use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::{Sender, channel};
use std::thread::{JoinHandle, spawn};
use std::time::Instant;

type TokenRequest = ();
type AcquisitionResult = Result<(String, Instant), String>;
/**
 * WARNING!: The original track of the exercise doesn't specify that the TokenAcquirer is Send.
 * Implementing this without that requirement though seems at first glance undoable using channels.
 */
type TokenAcquirer = dyn Fn() -> AcquisitionResult + Sync + Send;

enum Event {
    Get(Sender<AcquisitionResult>),
    TryGet(Sender<Option<String>>),
    Acquisition(AcquisitionResult),
    Drop,
}

enum State {
    Empty,
    Pending(Option<VecDeque<Sender<AcquisitionResult>>>),
    Valid((String, Instant)),
}

// TODO: check the required tests
struct Inner {
    sender: Option<Sender<Event>>,
    thread: Option<JoinHandle<()>>,
    acquirer: Option<JoinHandle<()>>,
}

impl Inner {
    pub fn new(acquire_token: Box<TokenAcquirer>) -> Self {
        let (sender, receiver) = channel::<Event>();
        let (token_req_snd, token_req_rx) = channel::<TokenRequest>();

        let request_handler = move || {
            let mut queue = Some(VecDeque::<Sender<AcquisitionResult>>::new());
            let mut state = State::Empty;
            while let Ok(event) = receiver.recv() {
                match event {
                    Event::Get(sender) => match &mut state {
                        State::Empty => {
                            token_req_snd.send(() as TokenRequest).unwrap();
                            state = State::Pending(queue.take());
                            queue.as_mut().unwrap().push_back(sender);
                        }
                        State::Pending(senders) => senders.as_mut().unwrap().push_back(sender),
                        State::Valid((token, expiry)) => {
                            if Instant::now() >= *expiry {
                                token_req_snd.send(() as TokenRequest).unwrap();
                                state = State::Pending(queue.take());
                                queue.as_mut().unwrap().push_back(sender);
                            } else {
                                sender.send(Ok((token.clone(), *expiry))).unwrap();
                            }
                        }
                    },
                    Event::TryGet(sender) => match &mut state {
                        State::Empty => sender.send(None).unwrap(),
                        State::Pending(_) => sender.send(None).unwrap(),
                        State::Valid((token, _)) => sender.send(Some(token.clone())).unwrap(),
                    },
                    Event::Acquisition(result) => match &mut state {
                        State::Pending(senders) => {
                            senders
                                .as_ref()
                                .unwrap()
                                .iter()
                                .for_each(|s| s.send(result.clone()).unwrap());
                            senders.as_mut().unwrap().clear();
                            queue = senders.take();
                            match result {
                                Ok(response) => state = State::Valid(response), // we assume non expired tokens are received
                                Err(_) => state = State::Empty,
                            }
                        }
                        _ => unreachable!(),
                    },
                    Event::Drop => {
                        drop(token_req_snd);
                        break;
                    }
                }
            }
        };
        let token_acquirer = {
            let sender = sender.clone();
            move || {
                while let Ok(_) = token_req_rx.recv() {
                    sender.send(Event::Acquisition(acquire_token())).unwrap();
                }
            }
        };
        Self {
            sender: Some(sender),
            thread: Some(spawn(request_handler)),
            acquirer: Some(spawn(token_acquirer)),
        }
    }

    pub fn get_token(&self) -> Result<String, String> {
        let (sender, receiver) = channel::<AcquisitionResult>();
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::Get(sender))
            .unwrap();
        match receiver.recv() {
            Ok(r) => match r {
                Ok((tok, _)) => Ok(tok),
                Err(e) => Err(e),
            },
            Err(_) => Err("Token Manager hang up".to_string()),
        }
    }

    pub fn try_get_token(&self) -> Option<String> {
        let (sender, receiver) = channel::<Option<String>>();
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::TryGet(sender))
            .unwrap();
        receiver.recv().map_err(|_| None as Option<String>).unwrap()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.sender.take().unwrap().send(Event::Drop).unwrap();
        self.acquirer.take().unwrap().join().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}

enum Lazy {
    Preallocated(Option<Box<TokenAcquirer>>),
    Loaded(Inner),
}

impl Lazy {
    fn is_loaded(&self) -> bool {
        match self {
            Lazy::Preallocated(_) => false,
            Lazy::Loaded(_) => true,
        }
    }
}

pub struct TokenManager(RefCell<Lazy>);

impl TokenManager {
    #[inline]
    fn ensure_loaded(&self) {
        if !self.0.borrow().is_loaded() {
            let acquire_token: Box<TokenAcquirer> = match &mut *self.0.borrow_mut() {
                Lazy::Preallocated(opt) => opt.take(),
                Lazy::Loaded(_) => unreachable!(),
            }
            .unwrap();
            self.0.replace(Lazy::Loaded(Inner::new(acquire_token)));
        }
    }

    pub fn new(acquire_token: Box<TokenAcquirer>) -> Self {
        Self(RefCell::new(Lazy::Preallocated(Some(acquire_token))))
    }

    pub fn get_token(&self) -> Result<String, String> {
        self.ensure_loaded();
        match &*self.0.borrow() {
            Lazy::Preallocated(_) => unreachable!(),
            Lazy::Loaded(inner) => inner.get_token(),
        }
    }

    pub fn try_get_token(&self) -> Option<String> {
        self.ensure_loaded();
        match &*self.0.borrow() {
            Lazy::Preallocated(_) => unreachable!(),
            Lazy::Loaded(inner) => inner.try_get_token(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::token_manager::mpsc_two_workers::TokenManager;

    #[test]
    fn tokne_manager_drops_correctly() {
        let dummy_acquirer = || Err("Failure".to_string());
        (0..100).map(|_| dummy_acquirer.clone()).for_each(|acq| {
            let _manager = TokenManager::new(Box::new(acq));
        });
        assert!(true);
    }
}
