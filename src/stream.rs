use slog;
use entice::Context;
use errors::*;
use telebot::bot;
use telebot::objects::*;
use telebot::functions::*;

use erased_serde::Serialize;

use futures::{future, Future};

const QUERY_REPLY_TEXT: &'static str = "I'm nominating you for invitation to \
                                        {group}.\n\nAfter pressing the button \
                                        below, you must also press the Start \
                                        button.";

pub struct Handler {
    logger: slog::Logger,
}

impl Handler {
    pub fn new(logger: slog::Logger) -> Handler {
        Handler { logger: logger }
    }

    pub fn dispatch(
        &self,
        tg: bot::RcBot,
        ctx: &Option<Context>,
        upd: Update,
    ) -> Box<Future<Item = (), Error = Error>> {
        if let Some(inline) = upd.inline_query {
            debug!(self.logger, "inline: {:?}", inline);
            return self.handle_inline_query(tg, inline);
        }

        if let Some(query) = upd.callback_query {
            debug!(self.logger, "callback: {:?}", query);
            return self.handle_callback_query(tg, query);
        }

        if let Some(msg) = upd.message {
            debug!(self.logger, "message: {:?}", msg);
            return self.handle_message(tg, msg, ctx);
        }

        debug!(self.logger, "Other Update: {:?}", upd);

        Box::from(future::ok(()))
    }

    fn handle_message(
        &self,
        _: bot::RcBot,
        msg: ::telebot::objects::Message,
        ctx: &Option<Context>,
    ) -> Box<Future<Item = (), Error = Error>> {
        let ctx = match ctx {
            &Some(ref x) => x,
            &None => {
                warn!(self.logger, "Got message update, but have no context");
                return Box::from(future::ok(()));
            }
        };

        if let Some(user) = msg.new_chat_member {
            if user.id == ctx.user.id {}
        }

        Box::from(future::ok(()))
    }

    fn handle_callback_query(
        &self,
        tg: bot::RcBot,
        query: CallbackQuery,
    ) -> Box<Future<Item = (), Error = Error>> {
        debug!(self.logger, "Got callback_query");

        Box::from(
            tg.answer_callback_query(query.id)
                .url("t.me/EnticeBot?start=hello")
                .send()
                .map(|_| ())
                .from_err(),
        )
    }

    fn handle_inline_query(
        &self,
        tg: bot::RcBot,
        query: InlineQuery,
    ) -> Box<Future<Item = (), Error = Error>> {
        let result: Vec<Box<Serialize>> = vec![
            Box::new(
                InlineQueryResultArticle::new(
                    "Test Group Name".into(),
                    Box::new(InputMessageContent::Text::new(
                        QUERY_REPLY_TEXT.into(),
                    )),
                ).reply_markup(InlineKeyboardMarkup::new(
                    vec![
                        vec![
                            InlineKeyboardButton::new(
                                "Accept Nomination".into(),
                            ).callback_data("http://wikipedia.org"),
                        ],
                    ],
                )),
            ),
        ];

        let logger = self.logger.clone();
        Box::from(
            tg.answer_inline_query(query.id, result)
                .is_personal(true)
                .cache_time(0) // TODO: Can probably set this higher
                .send()
                .map(move |_| debug!(logger, "Sent answer_inline_query"))
                .from_err(),
        )
    }
}
