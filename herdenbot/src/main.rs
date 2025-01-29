use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use warp::Filter;
use std::path::PathBuf;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::process::Command;

static SECRETDATA: OnceCell<serde_json::Value> = OnceCell::new();


static DECRYPT_ENABLED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));


async fn handle_decrypt(bot2: Arc<teloxide::Bot>) -> Result<impl warp::Reply, warp::Rejection> { 
        let do_decrypt = *DECRYPT_ENABLED.lock().unwrap();
        let chat_ids = SECRETDATA.get().expect("Unable to get secret data")["wichtel"]
                .as_object()
                .expect("Unable to get wichtel chat ids");
        let chat_id: i64 = chat_ids.get("Flo").expect("No flo?").as_number().expect("No flo in chat id").as_i64().unwrap();
        let chat_id = teloxide::types::ChatId(chat_id.into());
        bot2.send_message(chat_id, format!("Decrypt requested. Result was: {do_decrypt}"))
                        .await.unwrap(); 

        let key = SECRETDATA.get().and_then(|x| x["zfs_keys"].as_object()).and_then(|x| x["sami"].as_str()).expect("No key found").to_string();
        if do_decrypt {
      Ok(warp::reply::with_status(key, warp::http::StatusCode::OK))
    } else {
        Ok(warp::reply::with_status("Forbidden".to_string(), warp::http::StatusCode::FORBIDDEN))
    }
}
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    //get secret file from env
    let secret_file = std::env::var("SECRET_FILE").unwrap_or_else(|_| {
        "/secrets/herdenbot/bot.json".to_string()
    });
    log::info!("Starting command bot...");
    let data = std::fs::read_to_string(&secret_file)
        .expect(&format!("Unable to read file {}", secret_file));
    let data: serde_json::Value = serde_json::from_str(&data).expect("JSON was not well-formatted");
    let token = data["token"]
        .as_str()
        .expect("Token was not a string")
        .to_string();
    log::info!("Starting throw dice bot...");
    SECRETDATA.set(data).expect("Unable to set secret data");

    let bot = Arc::new(Bot::new(token));
    let bot2 = bot.clone();
    let bot_filter = warp::any().map(move || bot.clone());
    let hello = warp::path!("sami").and(warp::get()).and(bot_filter).and_then(handle_decrypt);

    let http_server = warp::serve(hello).run(([0,0,0,0],51822));

    let j1 = tokio::task::spawn(async {
        BotCommand::repl(bot2, answer).await;
    });
    let j2 = tokio::task::spawn( async {
        http_server.await;
    });
    tokio::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
        //exit process
        std::process::exit(0);
    });
    j1.await.unwrap();
    j2.await.unwrap();


    /* teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
    .await; */
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum BotCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start the valheim server")]
    StartValheim,
    #[command(description = "stop the valheim server")]
    StopValheim,
    #[command(description = "Weihnachtwichtel auswuerfeln")]
    Wichteln,
    #[command(description = "Decrypt")]
    Decrypt,
}

async fn answer(bot: Arc<Bot>, msg: Message, cmd: BotCommand) -> ResponseResult<()> {
    log::info!("Received {:?}", msg);
    match cmd {
        BotCommand::Help => {
            bot.send_message(msg.chat.id, BotCommand::descriptions().to_string())
                .await?
        }
        BotCommand::StartValheim => {
            bot.send_message(msg.chat.id, format!("Starting valheim..."))
                .await?;
            let output = Command::new("/run/wrappers/bin/sudo")
                .arg("/run/current-system/sw/bin/systemctl")
                .arg("start")
                .arg("valheim-server.service")
                .output()
                .await?;

            if output.status.success() {
                //write current timestamp to /home/herdenbot/last_start.timestamp
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                std::fs::write(
                    "/home/herdenbot/last_start.timestamp",
                    timestamp.to_string(),
                )?;
                bot.send_message(msg.chat.id, format!("success")).await?
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);

                bot.send_message(msg.chat.id, format!("error {}", error_message))
                    .await?
            }
        }
        BotCommand::StopValheim => {
            bot.send_message(msg.chat.id, format!("Stopping valheim"))
                .await?;
            let output = Command::new("/run/wrappers/bin/sudo")
                .arg("/run/current-system/sw/bin/systemctl")
                .arg("stop")
                .arg("valheim-server.service")
                .output()
                .await?;

            if output.status.success() {
                bot.send_message(msg.chat.id, format!("success")).await?
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);

                bot.send_message(msg.chat.id, format!("error {}", error_message))
                    .await?
            }
        }
        BotCommand::Wichteln => {
            use chrono::{Datelike, Utc};
            //make sure it's Florian F.
            //load chatids from secret file
            let chat_ids = SECRETDATA.get().expect("Unable to get secret data")["wichtel"]
                .as_object()
                .expect("Unable to get wichtel chat ids");
            if msg.chat.id.to_string()
                == format!(
                    "{}",
                    chat_ids
                        .get("Flo")
                        .expect("No flo?")
                        .as_number()
                        .expect("No flo in chat id")
                )
            {
                let wichtel_file: PathBuf =
                    format!("/secrets/herdenbot/wichtel_{}.json", Utc::now().year()).into();
                if !wichtel_file.exists() {
                    //wuerfeln..
                    let mut forbidden: HashSet<(String, String)> = HashSet::new();
                    for ab in SECRETDATA.get().expect("unable to get secret data")["forbidden"].as_array().expect("forbidden not arrays") {
                        forbidden.insert((ab[0].as_str().unwrap().to_string(), ab[1].as_str().unwrap().to_string()));
                    }
                    let wichtel = draw_wichtel(
                        &(chat_ids.keys().map(|x| x.to_string()).collect()),
                        &forbidden,
                    );
                    //save in json
                    std::fs::write(
                        &wichtel_file,
                        serde_json::to_string(&wichtel).expect("Unable to serialize wichtel"),
                    )?;
                    bot.send_message(msg.chat.id, format!("Wichteln gemacht, Datei geschrieben. Noch mal /wichteln sagen zum versenden")) .await?
                } else {
                    //load from file
                    let wichtel: HashMap<String, String> =
                        serde_json::from_str(&std::fs::read_to_string(wichtel_file)?)
                            .expect("Unable to deserialize wichtel");
                    for (wichtel_from, wichtel_to) in wichtel.iter() {
                        let receiver = format!("{}", chat_ids.get(wichtel_from).expect("No chat id for wichtel_from").as_number().expect("chat_id target was not a number"));
                        bot.send_message(
                            receiver,
                            format!("Du wichtelst fuer {}", wichtel_to),
                        ).await?;
                    }
                    bot.send_message(msg.chat.id, format!("Wichteln versendet"))
                        .await?
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!("Du bist nicht Florian und darfst das wichteln nicht triggern"),
                )
                .await?
            }
        }
        BotCommand::Decrypt => { 
let chat_ids = SECRETDATA.get().expect("Unable to get secret data")["wichtel"]
                .as_object()
                .expect("Unable to get wichtel chat ids");

            if msg.chat.id.to_string()
                == format!(
                    "{}",
                    chat_ids
                        .get("Flo")
                        .expect("No flo?")
                        .as_number()
                        .expect("No flo in chat id")
                ) {
                *DECRYPT_ENABLED.lock().unwrap() = true;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(150)).await;
                    println!("disabling decrypt");
                    *DECRYPT_ENABLED.lock().unwrap() = false;
                });
                bot.send_message(msg.chat.id, format!("Decrypt enabled"))
                        .await?
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!("Du bist nicht Florian und darfst das nicht"),
                )
                .await?
            }
        }
    };


    Ok(())
}

fn draw_wichtel(
    wichtel: &HashSet<String>,
    forbidden: &HashSet<(String, String)>,
) -> HashMap<String, String> {
    use rand::prelude::SliceRandom;

    let first_wichtel: Vec<String> = wichtel.iter().cloned().collect();
    let full_forbidden = forbidden
        .iter()
        .cloned()
        .chain(forbidden.iter().map(|(a, b)| (b.clone(), a.clone())))
        .collect::<HashSet<_>>();

    //randomize order of second_wichtel
    let mut rng = rand::thread_rng();
    let mut second_wichtel: Vec<String> = wichtel.iter().cloned().collect();
    let mut out: Option<Vec<(String, String)>> = None;
    while out.is_none() {
        second_wichtel.shuffle(&mut rng);
        out = Some(
            first_wichtel
                .iter()
                .map(|x| x.to_string())
                .zip(second_wichtel.iter().map(|x| x.to_string()))
                .collect(),
        );
        for (a, b) in out.as_ref().unwrap() {
            if full_forbidden.contains(&(a.to_owned(), b.to_owned())) {
                out = None;
                break;
            }
            if a == b {
                out = None;
                break;
            }
        }
    }
    let out = out.expect("No out?");
    out.into_iter().collect()
}
