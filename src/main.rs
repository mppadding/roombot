use serenity::client::Client;
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::prelude::*;
use serenity::prelude::{Context, EventHandler};

use serenity::model::channel::{
    ChannelType, Message, PermissionOverwrite, PermissionOverwriteType, Reaction,
};
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::model::permissions::Permissions;

use clokwerk::{Scheduler, TimeUnits};

use std::time::Duration;

mod bot;

use bot::database::DatabaseManager;
use bot::room::Room;

struct DBManager;

impl TypeMapKey for DBManager {
    type Value = DatabaseManager;
}

group!({
    name: "general",
    options: {},
    commands: [ping, test, sync],
});

use std::env;

struct Handler;

impl EventHandler for Handler {
    fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        if add_reaction.emoji.as_data() == "🔁" && add_reaction.user_id != 609399335205208064 {
            let mut data = ctx.data.write();
            let db = data.get_mut::<DBManager>().unwrap();

            let chn = db.message_get(add_reaction.message_id);

            match chn {
                Ok(channel_id) => {
                    db.room_prolong(channel_id)
                        .expect("Could not prolong channel");
                    db.message_delete(add_reaction.message_id.as_u64())
                        .expect("Could not delete message in redis");

                    add_reaction
                        .message(&ctx)
                        .unwrap()
                        .delete(&ctx)
                        .expect("Error deleting message");
                }
                Err(_) => {}
            }
        }
    }
}

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(
        &env::var("DISCORD_TOKEN").expect("Token not set in environment."),
        Handler,
    )
    .expect("Error creating client");

    {
        let mut data = client.data.write();
        data.insert::<DBManager>(bot::database::DatabaseManager::new());
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .group(&GENERAL_GROUP),
    );

    let mut scheduler = Scheduler::new();
    let cache_http = client.cache_and_http.clone();
    let data = client.data.clone();

    scheduler.every(5.seconds()).run(move || {
        let mut data = data.write();
        let db = data.get_mut::<DBManager>().unwrap();
        let rooms = db.room_list();

        let almost_expired_rooms: Vec<&Room> =
            rooms.iter().filter(|r| r.is_almost_expired()).collect();

        almost_expired_rooms.iter().for_each(|r| {
            let dm = r
                .creator()
                .create_dm_channel(&cache_http.http)
                .expect("Could not create DM channel");

            let dm_message = format!(
                "Your channel `{}` will be deleted in 1 hour. Press :repeat: to delay for 12 hours",
                r.name()
            );

            let dm = dm.send_message(&cache_http.http, |m| m.content(&dm_message));

            match dm {
                Ok(dm_msg) => {
                    db.message_add(dm_msg.id, r.id())
                        .expect("Could not add repeat message to redis");
                    let http = cache_http.http.clone();

                    dm_msg
                        .react(http, "🔁")
                        .expect("Could not add :repeat: to message");
                }
                Err(why) => println!("Error sending repeat: {}", why),
            };
        });
    });

    let thread_handle = scheduler.watch_thread(Duration::from_millis(100));

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write();
    let db = data.get_mut::<DBManager>().unwrap();
    println!("{:?}", db.room_list());

    msg.reply(&ctx, "Pong!")?;

    Ok(())
}

#[command]
fn test(ctx: &mut Context, msg: &Message) -> CommandResult {
    let id = match msg.guild_id {
        Some(x) => x,
        None => {
            msg.reply(ctx, "Command can only be used in guild")?;

            return Ok(());
        }
    };
    // Get the channel name from the message (only non <@...> string)
    let channel_name = msg.content.split_whitespace().skip(1).next().unwrap_or("");

    // Create voice channel
    let voice = id
        .create_channel(&ctx.http, |c| {
            c.name(channel_name)
                .category(ChannelId(636169359856893972))
                .kind(ChannelType::Voice)
        })
        .expect("Cannot create channel");

    // Permissions to deny everyone everything, except seeing the channel
    let everyone = PermissionOverwrite {
        allow: Permissions::READ_MESSAGES,
        deny: Permissions::all(),
        kind: PermissionOverwriteType::Role(RoleId(233637050220281856)),
    };

    // Set permissions
    voice
        .create_permission(&ctx, &everyone)
        .expect("Failed setting permissions for everyone");

    // List of users to allow
    let mut users: Vec<&u64> = msg.mentions.iter().map(|user| user.id.as_u64()).collect();
    users.insert(0, msg.author.id.as_u64());

    // Enable connect and speak for list of users
    for user in users {
        let perm = PermissionOverwrite {
            allow: Permissions::READ_MESSAGES | Permissions::CONNECT | Permissions::SPEAK,
            deny: Permissions::all(),
            kind: PermissionOverwriteType::Member(UserId(*user)),
        };

        voice
            .create_permission(&ctx, &perm)
            .expect("Failed setting permission for user");
    }

    let room = bot::room::Room::new(voice.id, channel_name.to_string(), msg.author.id);

    let mut data = ctx.data.write();
    let db = data.get_mut::<DBManager>().unwrap();
    db.room_add(room).expect("Could not create room");

    let dm_message = format!(
        "Your channel `{}` will be deleted in 1 hour. Press :repeat: to delay for 12 hours",
        channel_name
    );

    let dm = msg.author.direct_message(&ctx, |m| m.content(&dm_message));

    match dm {
        Ok(msg) => {
            db.message_add(msg.id, voice.id)
                .expect("Could not add repeat message to redis");
            msg.react(&ctx, "🔁")?
        }
        Err(why) => println!("Error sending repeat: {}", why),
    };

    Ok(())
}
