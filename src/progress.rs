use clout::status;
use indicatif::{ProgressBar, ProgressStyle};

pub struct Reporter {
    progress: ProgressBar,
}

impl Reporter {
    pub fn new(len: usize) -> Reporter {
        let progress = if clout::level() >= clout::Level::Info {
            ProgressBar::hidden()
        } else {
            ProgressBar::new(len as u64)
        };

        progress.set_style(ProgressStyle::default_bar()
            .template("{elapsed:.white} [{wide_bar:.green}] {pos:>4.white}/{len:4.white} (ETA {eta}) {msg:.cyan}")
            .expect("invalid template")
            .progress_chars("=> "));

        Reporter { progress }
    }

    pub fn inc(&self) {
        self.progress.inc(1);
    }

    pub fn report<T: AsRef<str>>(&self, msg: T) {
        if self.progress.is_hidden() {
            status!("{}", msg.as_ref());
        } else {
            self.progress.println(msg.as_ref());
        }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.progress.finish_and_clear();
    }
}
