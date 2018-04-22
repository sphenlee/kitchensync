use indicatif::{ProgressBar, ProgressStyle};
use atty;

pub struct Reporter {
    tty: bool,
    progress: ProgressBar
}

impl Reporter {
    pub fn new(len: usize) -> Reporter {
        let tty = atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr);

        let progress = ProgressBar::new(len as u64);

        progress.set_style(ProgressStyle::default_bar()
            .template("{elapsed:.white} [{wide_bar:.green}] {pos:>4.white}/{len:4.white} (ETA {eta}) {msg:.cyan}")
            .progress_chars("=> "));

        Reporter {
            tty,
            progress
        }
    }

    pub fn inc(&self) {
        self.progress.inc(1);
    }

    pub fn report<T: AsRef<str>>(&self, msg: T) {
        if !self.tty {
            println!("{}", msg.as_ref());
        } else {
            let mut strmsg = msg.as_ref().to_owned();
            strmsg.truncate(40);
            self.progress.set_message(&strmsg);
        }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.progress.finish_and_clear();
    }
}
