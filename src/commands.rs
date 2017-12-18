use entice::Context;
use templates;
use telebot::{self, bot};
use telebot::objects::Message;
use telebot::functions::FunctionMessage;
use slog;

use futures::{future, Future, Stream};

use std::rc::Rc;
use std::cell::RefCell;

pub fn register_all(logger: slog::Logger,
                    tg: &bot::RcBot,
                    ctx: Rc<RefCell<Option<Context>>>) {
    register(Start::new(tg.clone(), logger), tg, ctx.clone());
}

fn register<T: Command>(
    mut cmd: T,
    tg: &bot::RcBot,
    ctx: Rc<RefCell<Option<Context>>>,
) {
    let hndl = tg.new_cmd(T::NAME).and_then(
        move |(tg, msg)| match *ctx.borrow() {
            None => Box::from(tg.message(
                msg.chat.id,
                "Not ready yet :(".to_owned(),
            ).send().map(|_| ())),
            Some(ref x) => cmd.handle(x, msg),
        },
    );

    tg.register(hndl);
}

trait Command: 'static {
    const NAME: &'static str;

    fn new(tg: bot::RcBot, logger: slog::Logger) -> Self;

    fn handle(
        &mut self,
        &Context,
        Message,
    ) -> Box<Future<Item = (), Error = telebot::Error>>;
}

struct Start {
    tg: bot::RcBot,
    logger: slog::Logger,
}

impl Command for Start {
    const NAME: &'static str = "/start";

    fn new(tg: bot::RcBot, logger: slog::Logger) -> Self {
        Start {
            tg: tg,
            logger: logger,
        }
    }

    fn handle(
        &mut self,
        ctx: &Context,
        msg: Message,
    ) -> Box<Future<Item = (), Error = telebot::Error>> {
        if msg.chat.kind != "private" {
            return Box::from(future::ok(()));
        }

        let text = ctx.templates.render(templates::REPLY_START, &json!({
            "username": ctx.user.username,
        })).unwrap();

        Box::from(self.tg.message(msg.chat.id, text).send().map(|_| ()))
    }
}
