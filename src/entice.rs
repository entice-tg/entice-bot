use stream::Handler;
use commands;

use slog;

use errors::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

use settings::Settings;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use tokio_core::reactor::Core;

use telebot::bot::RcBot;
use telebot::objects::{Update, User};
use telebot::functions::FunctionGetMe;

use futures::{future, Future, IntoFuture, Stream};
use futures::sync::mpsc::{channel, Receiver, Sender};

pub type JoinHandle = ::std::thread::JoinHandle<Result<()>>;

enum StreamItem {
    Command(Command),
    Telegram(RcBot, Update),
}

pub struct Context {
    pub user: User,
    pub db: PgConnection,
}

struct EventLoop {
    tg: RcBot,
    event_loop: Core,
    receiver: Receiver<Command>,
    context: Rc<RefCell<Option<Context>>>,
    update_handler: Rc<Handler>,
    logger: slog::Logger,
    settings: Settings,
}

impl EventLoop {
    pub fn new(
        logger: slog::Logger,
        settings: Settings,
        receiver: Receiver<Command>,
    ) -> Result<EventLoop> {
        let ev = Core::new().chain_err(|| "unable to create event loop")?;
        let tg = RcBot::new(ev.handle(), &settings.telegram_bot.auth_token)
            .update_interval(settings.telegram_bot.update_interval);

        Ok(EventLoop {
            event_loop: ev,
            tg: tg.clone(),
            receiver: receiver,
            context: Rc::from(RefCell::from(None)),
            update_handler: Rc::from(Handler::new(logger.clone(), tg.clone())),
            logger: logger,
            settings: settings,
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        commands::register_all(&self.tg);
        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        let tg = &self.tg;

        let db = PgConnection::establish(&self.settings.database.url)
            .chain_err(|| "unable to connect to database")?;

        let log1 = self.logger.clone();
        let log2 = self.logger.clone();
        let ctx = self.context.clone();
        self.event_loop.handle().spawn(
            tg.get_me()
                .send()
                .and_then(move |(_, user)| {
                    info!(log1, "My username: {:?}", &user.username);
                    *ctx.borrow_mut() = Some(Context { user: user, db: db });
                    Ok(())
                })
                .or_else(move |x| {
                    error!(log2, "unable to get_me(): {}", x);
                    Ok(())
                }),
        );

        let updates = tg.get_stream()
            .map(|(tg, u)| StreamItem::Telegram(tg, u))
            .from_err();

        let commands = self.receiver
            .map(|x| StreamItem::Command(x))
            .map_err(|_| Error::from("command error"));

        let ctx = self.context.clone();
        let handler = self.update_handler.clone();
        let stream = commands
            .select(updates)
            .take_while(|x| {
                Ok(match x {
                    &StreamItem::Command(Command::Stop) => false,
                    _ => true,
                })
            })
            .and_then(|x| match x {
                StreamItem::Command(Command::Stop) => {
                    Box::from(future::result(Err(Error::from("Stop"))))
                        as Box<Future<Item = (), Error = Error>>
                }

                StreamItem::Telegram(_, u) => {
                    handler.dispatch(&*ctx.borrow(), u)
                }
            });

        self.event_loop
            .run(stream.for_each(|_| Ok(())).into_future())
            .chain_err(|| "error in main loop")
    }
}

#[derive(Debug)]
enum Command {
    Stop,
}

pub struct EnticeBot {
    sender: Sender<Command>,
    receiver: Option<Receiver<Command>>,
}

impl EnticeBot {
    pub fn new() -> EnticeBot {
        let (sender, receiver) = channel(0);

        EnticeBot {
            sender: sender,
            receiver: Some(receiver),
        }
    }

    pub fn start(
        &mut self,
        logger: slog::Logger,
        settings: Settings,
    ) -> Result<JoinHandle> {
        let receiver = match self.receiver.take() {
            Some(x) => x,
            None => bail!(ErrorKind::AlreadyStarted),
        };

        Ok(thread::spawn(|| Self::run(logger, settings, receiver)))
    }

    pub fn stop(&mut self) -> Result<()> {
        match self.sender.try_send(Command::Stop) {
            Ok(_) => Ok(()),
            Err(ref x) if x.is_disconnected() => {
                bail!(ErrorKind::AlreadyStopped)
            }
            x => x.chain_err(|| ""),
        }
    }

    fn run(
        logger: slog::Logger,
        settings: Settings,
        receiver: Receiver<Command>,
    ) -> Result<()> {
        let mut lp = EventLoop::new(logger, settings, receiver)?;

        lp.setup()?;
        lp.run()
    }
}
