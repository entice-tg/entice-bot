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

pub fn dispatch(
    logger: slog::Logger,
    tg: bot::RcBot,
    _ctx: &Option<Context>,
    upd: Update,
) -> Box<Future<Item = (), Error = Error>> {
    if let Some(inline) = upd.inline_query {
        debug!(logger, "inline: {:?}", inline);
        return handle_inline_query(logger, tg, inline);
    }

    if let Some(query) = upd.callback_query {
        debug!(logger, "callback: {:?}", query);
        return handle_callback_query(logger, tg, query);
    }

    debug!(logger, "Other Update: {:?}", upd);

    Box::from(future::ok(()))
}

fn handle_callback_query(
    logger: slog::Logger,
    tg: bot::RcBot,
    query: CallbackQuery,
) -> Box<Future<Item = (), Error = Error>> {
    debug!(logger, "Got callback_query");

    Box::from(
        tg.answer_callback_query(query.id)
            .url("t.me/EnticeBot?start=hello")
            .send()
            .map(|_| ())
            .from_err(),
    )
}

fn handle_inline_query(
    logger: slog::Logger,
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
            ).reply_markup(InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::new("Accept Nomination".into())
                        .callback_data("http://wikipedia.org"),
                ],
            ])),
        ),
    ];

    Box::from(
        tg.answer_inline_query(query.id, result)
        .is_personal(true)
        .cache_time(0) // TODO: Can probably set this higher
        .send()
        .map(move |_| debug!(logger, "Sent answer_inline_query"))
        .from_err(),
    )
}
