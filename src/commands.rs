use telebot::{self, bot};
use telebot::objects::Message;
use telebot::functions::FunctionMessage;

use futures::{Future, Stream};

pub fn register_all(tg: &bot::RcBot) {
    Start::register(tg);
}

trait Command: 'static {
    const NAME: &'static str;

    fn register(tg: &bot::RcBot) {
        let hndl = tg.new_cmd(Self::NAME).and_then(Self::handle);

        tg.register(hndl);
    }

    fn handle(
        (bot::RcBot, Message),
    ) -> Box<Future<Item = (bot::RcBot, Message), Error = telebot::Error>>;
}

struct Start;

impl Start {
    const INTRO: &'static str = "Hello! I'm @EnticeBot.\n\nI help manage \
                                 inviting new users to groups. If you have a \
                                 channel you'd like me to help you with, try \
                                 /register. Otherwise, you might want to try \
                                 /help.";

    const NOM: &'static str = "Hello! I'm @EnticeBot.\n\nYou've been \
                               nominated for invitation to {group}, and will \
                               receive a message here if approved.";

    fn handle_nomination(
        tg: bot::RcBot,
        msg: Message,
    ) -> Box<Future<Item = (bot::RcBot, Message), Error = telebot::Error>> {
        Box::from(tg.message(msg.chat.id, Self::NOM.to_owned()).send())
    }

    fn handle_introduction(
        tg: bot::RcBot,
        msg: Message,
    ) -> Box<Future<Item = (bot::RcBot, Message), Error = telebot::Error>> {
        Box::from(tg.message(msg.chat.id, Self::INTRO.to_owned()).send())
    }
}

impl Command for Start {
    const NAME: &'static str = "/start";

    fn handle(
        (tg, msg): (bot::RcBot, Message),
    ) -> Box<Future<Item = (bot::RcBot, Message), Error = telebot::Error>> {
        // TODO: Stop fighting the borrow checker here
        let len = match msg.text {
            None => 0,
            Some(ref txt) => txt.len(),
        };

        if len > 0 {
            Self::handle_nomination(tg, msg)
        } else {
            Self::handle_introduction(tg, msg)
        }
    }
}
