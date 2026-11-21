use std::thread;
use std::time::Duration;

use iced::Subscription;
use iced::futures::channel::mpsc::{self, UnboundedReceiver};

pub(super) fn every(interval: Duration) -> Subscription<()> {
    Subscription::run_with(interval, ticks)
}

fn ticks(interval: &Duration) -> UnboundedReceiver<()> {
    let interval = *interval;
    let (sender, receiver) = mpsc::unbounded();
    thread::spawn(move || {
        loop {
            thread::sleep(interval);
            if sender.unbounded_send(()).is_err() {
                break;
            }
        }
    });
    receiver
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use iced::futures::StreamExt;
    use iced::futures::executor::block_on;

    use crate::app::ticks::ticks;

    #[test]
    fn a_tick_comes_after_each_interval() {
        let started = Instant::now();
        let mut ticks = ticks(&Duration::from_millis(20));
        assert_eq!(block_on(ticks.next()), Some(()));
        assert_eq!(block_on(ticks.next()), Some(()));
        assert!(started.elapsed() >= Duration::from_millis(40));
    }
}
