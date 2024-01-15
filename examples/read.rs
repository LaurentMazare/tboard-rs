use anyhow::Result;
use clap::Parser;
use timens::{Span, Time};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    path: std::path::PathBuf,
}

struct ValueP(Option<tb::tensorboard::summary::value::Value>);

impl std::fmt::Display for ValueP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use tb::tensorboard::summary::value::Value;
        match &self.0 {
            None => Ok(()),
            Some(v) => match v {
                Value::SimpleValue(v) => v.fmt(f),
                Value::Histo(v) => write!(f, "{:?}", v),
                Value::Audio(v) => write!(f, "{:?}", v),
                Value::Tensor(v) => write!(f, "{:?}", v),
                Value::Image(v) => write!(f, "{:?}", v),
                Value::ObsoleteOldStyleHistogram(v) => write!(f, "{:?}", v),
            },
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let reader = std::fs::File::open(args.path)?;
    let reader = tb::SummaryReader::new(reader);
    for event in reader {
        let event = event?;
        let wall_time = Time::of_span_since_epoch(Span::of_sec(event.wall_time));
        print!("{:32} step: {:8}    ", wall_time.to_string(), event.step);
        match event.what {
            Some(tb::tensorboard::event::What::Summary(summary)) => {
                for value in summary.value {
                    print!(" {}: {}", value.tag, ValueP(value.value))
                }
            }
            _ => print!(" {:?}", event.what),
        }
        println!();
    }
    Ok(())
}
