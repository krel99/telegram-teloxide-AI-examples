use dotenv::dotenv;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

const MODEL_OPENAI: &str = "gpt-4o-mini";
const OPENAI_ENV_NAME: &str = "OPENAI_API_KEY";

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// Default - instance creator
// Debug - allows me to log State to console
#[derive(Clone, Default)] // why clone necessary, what is default, when is debug used
pub enum State {
    #[default]
    Start,
    Conversation {
        messages: ChatRequest,
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

    println!("DISPATCHER ended");

    Ok(())
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    println!("fn START");
    match msg.text() {
        Some(text) => {
            let existing_dialogue = dialogue.get().await;

            let mut chat_req = if let Ok(Some(State::Conversation { messages })) = existing_dialogue
            {
                let mut new_messages = messages.messages.clone();
                new_messages.push(ChatMessage::user(text));
                ChatRequest::new(new_messages)
            } else {
                ChatRequest::new(vec![
                    ChatMessage::system("Answer on the level of A1 speaker, then make open-ended statement or ask question."),
                    ChatMessage::user(text),
                ])
            };

            // let mut chat_req = ChatRequest::new(vec![
            //     ChatMessage::system("Answer on the level of A1 speaker, then make open-ended statement or ask question."),
            //     ChatMessage::user(text),
            // ]);

            // To print a Vec of ChatMessages, we iterate through and print each

            let client = Client::default();
            let model = MODEL_OPENAI;
            let env_name = OPENAI_ENV_NAME;
            // Skip if does not have the environment name set
            if !env_name.is_empty() && std::env::var(env_name).is_err() {
                println!("===== Skipping model: {model} (env var not set: {env_name})");
            }
            let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
            let res = chat_res.content_text_into_string().unwrap_or_default();
            bot.send_message(msg.chat.id, res.clone()).await?;
            chat_req.messages.push(ChatMessage::assistant(res));

            dialogue
                .update(State::Conversation {
                    messages: chat_req.clone(),
                })
                .await?;

            for message in &chat_req.messages {
                println!("{:?}", message);
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}
