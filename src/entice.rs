use stream;
use commands;

use errors::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

use settings::Settings;

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

#[derive(Debug)]
pub struct Context {
    pub user: User,
}

struct EventLoop {
    tg: RcBot,
    event_loop: Core,
    receiver: Receiver<Command>,
    context: Rc<RefCell<Option<Context>>>,
}

impl EventLoop {
    pub fn new(
        settings: Settings,
        receiver: Receiver<Command>,
    ) -> Result<EventLoop> {
        let ev = Core::new().chain_err(|| "unable to create event loop")?;
        let tg = RcBot::new(ev.handle(), &settings.telegram_bot.auth_token)
            .update_interval(settings.telegram_bot.update_interval);

        Ok(EventLoop {
            event_loop: ev,
            tg: tg,
            receiver: receiver,
            context: Rc::from(RefCell::from(None)),
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        commands::register_all(&self.tg);
        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        let tg = &self.tg;

        let ctx = self.context.clone();
        self.event_loop.handle().spawn(
            tg.get_me()
                .send()
                .and_then(move |(_, user)| {
                    *ctx.borrow_mut() = Some(Context { user: user });
                    Ok(())
                })
                .or_else(|x| Ok(println!("error getting me: {}", x))),
        );

        let updates = tg.get_stream()
            .map(|(tg, u)| StreamItem::Telegram(tg, u))
            .from_err();

        let commands = self.receiver
            .map(|x| StreamItem::Command(x))
            .map_err(|_| Error::from("command error"));

        let ctx = self.context.clone();
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

                StreamItem::Telegram(t, u) => stream::dispatch(t, &*ctx.borrow(), u),
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

    pub fn start(&mut self, settings: Settings) -> Result<JoinHandle> {
        let receiver = match self.receiver.take() {
            Some(x) => x,
            None => bail!(ErrorKind::AlreadyStarted),
        };

        Ok(thread::spawn(|| Self::run(settings, receiver)))
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

    fn run(settings: Settings, receiver: Receiver<Command>) -> Result<()> {
        let mut lp = EventLoop::new(settings, receiver)?;

        lp.setup()?;
        lp.run()
    }
}
