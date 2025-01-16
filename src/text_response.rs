use dotenv::dotenv;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

const MODEL_OPENAI: &str = "gpt-4o-mini";
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
    // -- de/activate models/providers
    (MODEL_OPENAI, "OPENAI_API_KEY"),
];

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// Default - instance creator
// Debug - allows me to log State to console
#[derive(Clone, Default)] // why clone necessary, what is default, when is debug used
pub enum State {
    #[default]
    Start,
    Question {
        fullQuestion: String,
    },
}

#[tokio::main] // how comes I don't need to import this?
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    dotenv().ok(); // creates env variables from env

    // let openai_key = env::var("OPENAI_TX").unwrap();
    let bot = Bot::from_env();

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start)),
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let chat_req = ChatRequest::new(vec![
                ChatMessage::system("Answer in one sentence"),
                ChatMessage::user(text),
            ]);
            let client = Client::default();
            for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
                // Skip if does not have the environment name set
                if !env_name.is_empty() && std::env::var(env_name).is_err() {
                    println!("===== Skipping model: {model} (env var not set: {env_name})");
                    continue;
                }
                let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
                let res = chat_res.content_text_into_string().unwrap_or_default();
                bot.send_message(msg.chat.id, res).await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}
