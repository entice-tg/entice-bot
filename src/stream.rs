use slog;
use diesel;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::prelude::*;
use templates;
use entice::Context;
use errors::*;
use telebot::bot;
use telebot::objects::*;
use telebot::functions::*;

use models::{Chat as EnticeChat, NewChat as NewEnticeChat};

use erased_serde::Serialize;

use futures::{future, Future};

const QUERY_REPLY_TEXT: &'static str = "I'm nominating you for invitation to \
                                        {group}.\n\nAfter pressing the button \
                                        below, you must also press the Start \
                                        button.";

pub struct Handler {
    logger: slog::Logger,
    tg: bot::RcBot,
}

impl Handler {
    pub fn new(logger: slog::Logger, tg: bot::RcBot) -> Handler {
        Handler {
            logger: logger,
            tg: tg,
        }
    }

    pub fn dispatch(
        &self,
        ctx: &Option<Context>,
        upd: Update,
    ) -> Box<Future<Item = (), Error = Error>> {
        let ctx = match ctx {
            &None => {
                warn!(self.logger, "no context yet");
                return Box::from(future::ok(()));
            }
            &Some(ref x) => x,
        };

        if let Some(inline) = upd.inline_query {
            debug!(self.logger, "inline: {:?}", inline);
            return self.handle_inline_query(inline, ctx);
        }

        if let Some(query) = upd.callback_query {
            debug!(self.logger, "callback: {:?}", query);
            return self.handle_callback_query(query);
        }

        if let Some(msg) = upd.message {
            return self.handle_message(msg, ctx);
        }

        debug!(self.logger, "Other Update: {:?}", upd);

        Box::from(future::ok(()))
    }

    fn handle_message(
        &self,
        msg: ::telebot::objects::Message,
        ctx: &Context,
    ) -> Box<Future<Item = (), Error = Error>> {
        let matches = if let Some(ref user) = msg.new_chat_member {
            user.id == ctx.user.id
        } else {
            false
        };

        if matches {
            return self.handle_join_chat(msg, ctx);
        }

        let matches = if let Some(ref user) = msg.left_chat_member {
            user.id == ctx.user.id
        } else {
            false
        };

        if matches {
            return self.handle_left_chat(msg, ctx);
        }

        Box::from(future::ok(()))
    }

    fn handle_left_chat(
        &self,
        msg: ::telebot::objects::Message,
        ctx: &Context,
    ) -> Box<Future<Item = (), Error = Error>> {
        use schema::chats::dsl::*;

        debug!(self.logger, "Left Chat: {:?}", msg);

        let chat = &msg.chat;
        let result =
            diesel::delete(chats.filter(id.eq(chat.id))).execute(&ctx.db);

        if let Err(x) = result {
            error!(self.logger, "Unable to delete chat: {}", x);
        } else {
            let chat_title = match chat.title {
                Some(ref x) => x.as_str(),
                None => "",
            };
            info!(self.logger, "Left Chat: {} ({})", chat_title, chat.id);
        }

        Box::from(future::ok(()))
    }

    fn handle_join_chat(
        &self,
        msg: ::telebot::objects::Message,
        ctx: &Context,
    ) -> Box<Future<Item = (), Error = Error>> {
        debug!(self.logger, "Join Chat: {:?}", msg);

        let title = msg.chat.title.unwrap_or(String::default());
        let new_chat = NewEnticeChat {
            id: msg.chat.id,
            title: title.as_str(),
            description: "", // TODO: Get description
        };

        {
            use schema::chats;
            let chat: EnticeChat = match diesel::insert_into(chats::table)
                .values(&new_chat)
                .get_result(&ctx.db)
            {
                Ok(x) => x,
                Err(DieselError::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    error!(
                        self.logger,
                        "Aready joined chat: {} ({})",
                        new_chat.title,
                        new_chat.id
                    );
                    // TODO: Query and provide the chat, instead of stopping.
                    return Box::from(future::ok(()));
                }
                Err(e) => return Box::from(future::err(e.into())),
            };

            info!(self.logger, "Joined Chat: {} ({})", chat.title, chat.id);

            if msg.chat.kind == "private" {
                return Box::from(future::ok(()));
            }

        }

        let text = ctx.templates.render(templates::JOIN, &json!({
            "username": ctx.user.username,
        })).unwrap();

        Box::from(self.tg.message(msg.chat.id, text).send().map(|_| ()).from_err())
    }

    fn handle_callback_query(
        &self,
        query: CallbackQuery,
    ) -> Box<Future<Item = (), Error = Error>> {
        debug!(self.logger, "Got callback_query");

        Box::from(
            self.tg
                .answer_callback_query(query.id)
                .url("t.me/EnticeBot?start=hello")
                .send()
                .map(|_| ())
                .from_err(),
        )
    }

    fn handle_inline_query(
        &self,
        query: InlineQuery,
        ctx: &Context,
    ) -> Box<Future<Item = (), Error = Error>> {
        let chats = {
            use schema::chats::dsl::*;
            chats.load::<EnticeChat>(&ctx.db)
        };

        let chats = match chats {
            Ok(x) => x,
            Err(e) => {
                error!(self.logger, "unable to load chats: {}", e);
                return Box::from(future::ok(()));
            }
        };

        let logger = self.logger.clone();
        let tg = self.tg.clone();
        let tg2 = self.tg.clone();
        let query_id = query.id.clone();
        Box::from(
            future::join_all(chats.into_iter().map(move |chat| {
                // TODO: Swallow errors so they don't cancel all futures
                let title = chat.title.clone();
                tg.get_chat_member(chat.id, query.from.id)
                    .send()
                    .map(|(tg, mem)| (tg, mem, title))
            })).and_then(move |results| {
                let mut articles: Vec<Box<Serialize>> = Vec::new();

                for &(_, ref result, ref title) in results.iter() {
                    match result.status.as_str() {
                        "creator" | "administrator" | "member" => (),
                        _ => continue,
                    }
                    debug!(logger, "Got chat member: {:?}", result);

                    let article = Box::new(
                        InlineQueryResultArticle::new(
                            title.clone(),
                            Box::new(InputMessageContent::Text::new(
                                QUERY_REPLY_TEXT.into(),
                            )),
                        ).reply_markup(
                            InlineKeyboardMarkup::new(vec![
                                vec![
                                    InlineKeyboardButton::new(
                                        "Accept Nomination".into(),
                                    ).callback_data("http://wikipedia.org"),
                                ],
                            ]),
                        ),
                    );
                    articles.push(article);
                }
                let logger = logger.clone();
                Box::from(
                    tg2.answer_inline_query(query_id, articles)
                    .is_personal(true)
                    .cache_time(0) // TODO: Can probably set this higher
                    .send()
                    .map(move |_| debug!(logger, "Sent answer_inline_query"))
                    .from_err(),
                )
            })
                .or_else(|e| Err(Error::from(e))),
        )
    }
}
