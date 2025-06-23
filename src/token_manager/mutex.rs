use std::{
    sync::{Condvar, Mutex},
    time::Instant,
};

type Token = String;
type TokenAcquirer = dyn Fn() -> Result<(Token, Instant), String> + 'static;

#[derive(PartialEq)]
enum State {
    Empty,
    Pending,
    Valid((Token, Instant)),
}

pub struct TokenManager {
    state: (Mutex<State>, Condvar),
    acquirer: Mutex<Box<TokenAcquirer>>,
}

impl TokenManager {
    pub fn new(acquirer: Box<TokenAcquirer>) -> Self {
        Self {
            state: (Mutex::new(State::Empty), Condvar::new()),
            acquirer: Mutex::new(acquirer),
        }
    }

    pub fn get(&self) -> Result<Token, ()> {
        let mut guard = self.state.0.lock().unwrap();
        let mut state = &mut *guard;
        match state {
            State::Empty => {
                let acquirer_guard = self.acquirer.lock().unwrap();
                *state = State::Pending;
                drop(guard);
                let res = acquirer_guard(); // this might be slow
                guard = self.state.0.lock().unwrap();
                drop(acquirer_guard);
                state = &mut *guard;
                let mut handle_error = || {
                    *state = State::Empty;
                    self.state.1.notify_all();
                    Err(())
                };
                match res {
                    Err(_) => handle_error(),
                    Ok((_, expiry)) if expiry <= Instant::now() => handle_error(),
                    Ok((token, expiry)) => {
                        *state = State::Valid((token.clone(), expiry));
                        self.state.1.notify_all();
                        Ok(token)
                    }
                }
            }
            State::Pending => {
                guard = self
                    .state
                    .1
                    .wait_while(guard, |s| *s == State::Pending)
                    .unwrap();
                drop(guard);
                self.get()
            }
            State::Valid((_, expiry)) if *expiry <= Instant::now() => {
                *state = State::Empty;
                drop(guard);
                self.get()
            }
            State::Valid((token, _)) => Ok(token.clone()),
        }
    }

    pub fn try_get(&self) -> Option<Token> {
        let state = &*self.state.0.lock().unwrap();
        match state {
            State::Valid((token, expiry)) if *expiry > Instant::now() => Some(token.clone()),
            _ => None,
        }
    }
}
