use crate::{
    client::Client,
    config::Config,
    fd_set::{FdSet, fd_from_key, key},
    pre_sink::{PreSink, Verdict},
    pre_source::PreSource,
    tcp_listener::TcpListener,
    url_ext::UrlExt,
};
use anyhow::Result;
use mpclipboard_shared::{
    EventLoopFdResult, EventLoopWithTimer, ID, Message, Request, Store, Wants, prelude::*,
};
use std::{collections::HashMap, os::fd::AsRawFd};

pub struct MainLoop {
    event_loop: EventLoopWithTimer,
    now: u64,
    config: Config,
    store: Store,

    listener: TcpListener,
    pre_sources: FdSet<20, PreSource>,
    pre_sinks: FdSet<20, PreSink>,
    clients: FdSet<20, Client>,
}

impl MainLoop {
    pub(crate) fn new(config: Config) -> Result<Self> {
        let event_loop = EventLoopWithTimer::new()?;
        let listener = TcpListener::new(config.url.resolve()?, &event_loop)?;

        let pre_sources = FdSet::<20, PreSource>::new();
        let pre_sinks = FdSet::<20, PreSink>::new();
        let clients = FdSet::<20, Client>::new();

        Ok(Self {
            listener,
            event_loop,
            now: 0,
            config,
            store: Store::empty(),

            pre_sources,
            pre_sinks,
            clients,
        })
    }

    fn poll(&mut self) -> (Option<u64>, HashMap<i32, EventLoopFdResult>) {
        let polled = self
            .event_loop
            .wait(None)
            .unwrap_or_else(|err| unreachable!("failed to poll: {err}"));

        let mut revents = HashMap::<i32, EventLoopFdResult>::new();
        for (key, fd_revents) in polled.fds() {
            let revents = revents.entry(fd_from_key(key)).or_default();
            revents.readable |= fd_revents.readable;
            revents.writable |= fd_revents.writable;
            revents.has_error |= fd_revents.has_error;
        }

        (polled.time, revents)
    }

    pub(crate) fn poll_and_process_events(&mut self) {
        let (time, mut revents) = self.poll();
        let listener = revents.remove(&self.listener.as_raw_fd());

        for (fd, revents) in revents {
            if let Some(source) = self.pre_sources.remove(fd) {
                self.on_pre_source_event(source, revents);
            } else if let Some(sink) = self.pre_sinks.remove(fd) {
                self.on_pre_sink_event(sink, revents);
            } else if let Some(client) = self.clients.remove(fd) {
                self.on_client_event(client, revents);
            }
        }

        if let Some(now) = time {
            self.on_tick(now);
        }
        if let Some(revents) = listener {
            self.on_listener_event(revents);
        }
    }

    fn on_tick(&mut self, now: u64) {
        self.now = now;
        log::trace!("tick {}", self.now);

        self.pre_sources.reap(self.now);
        self.pre_sinks.reap(self.now);
    }

    fn on_listener_event(&mut self, revents: EventLoopFdResult) {
        let fd = match self.listener.accept(revents) {
            Ok(Some(fd)) => fd,
            Ok(None) => return,
            Err(err) => {
                log::error!("failed to accept(): {err:?}");
                return;
            }
        };
        let key = key(&fd);
        let fd = match self.event_loop.register(fd, key, Wants::Read) {
            Ok(fd) => fd,
            Err(err) => {
                log::error!("failed to register accepted fd: {err}");
                return;
            }
        };
        let source = PreSource::new(fd, self.now);
        log::trace!("[{source}] new source");
        self.pre_sources.insert(source);
    }

    fn on_pre_source_event(&mut self, source: PreSource, revents: EventLoopFdResult) {
        match source.on_poll_event(revents) {
            Err(err) => log::error!("{err:?}"),
            Ok(Pending(source)) => self.pre_sources.insert(source),
            Ok(Done((Request { token, id }, leftover, fd))) => {
                let verdict = if token == self.config.token {
                    Verdict::Accepted { id, leftover }
                } else {
                    log::info!("[{id}] invalid token={token:?}");
                    Verdict::Unauthorized { id }
                };

                let sink = match PreSink::new(fd, verdict, self.now) {
                    Ok(sink) => sink,
                    Err(err) => {
                        log::error!("{err:?}");
                        return;
                    }
                };
                log::info!("[{}] promoting to {sink}", sink.id());
                self.pre_sinks.insert(sink);
            }
        }
    }

    fn on_pre_sink_event(&mut self, sink: PreSink, revents: EventLoopFdResult) {
        match sink.on_poll_event(revents) {
            Err(err) => log::error!("{err:?}"),
            Ok(Pending(sink)) => self.pre_sinks.insert(sink),
            Ok(Done((Verdict::Unauthorized { id }, _fd))) => {
                log::info!("[{id}] rejected as unauthorized");
            }
            Ok(Done((Verdict::Accepted { id, leftover }, fd))) => {
                let mut client = match Client::new(fd, id, &leftover) {
                    Ok(client) => client,
                    Err(err) => {
                        log::error!("{err:?}");
                        return;
                    }
                };
                log::info!("[{}] promoting to {client}", client.id());
                if let Some(message) = self.store.current()
                    && let Err(err) = client.push(message)
                {
                    log::error!("{err:?}");
                    return;
                }
                self.clients.insert(client);
            }
        }
    }

    fn on_client_event(&mut self, client: Client, revents: EventLoopFdResult) {
        match client.on_poll_event(revents) {
            Err(err) => log::error!("{err:?}"),
            Ok(Pending(client)) => self.clients.insert(client),
            Ok(Done((message, client))) => {
                if self.store.add(&message) {
                    log::info!("broadcasting {message:?}");
                    self.broadcast(&message, client.id());
                }

                self.clients.insert(client);
            }
        }
    }

    fn broadcast(&mut self, message: &Message, sender_id: &ID) {
        for client in self
            .clients
            .fds_mut()
            .filter(|client| client.id() != sender_id)
        {
            if let Err(err) = client.push(message) {
                log::error!("{err:?}");
            }
        }
    }
}
