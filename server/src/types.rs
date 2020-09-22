use tokio::sync::mpsc;

pub type HandleSender = mpsc::UnboundedSender<String>;