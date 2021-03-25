use clap::Clap;

#[derive(Clap)]
#[clap(author = "Alex Plate <AlexPl292@gmail.com>", version = "0.1.0")]
pub struct Opts {
    /// List of chats that are going to be saved. All chats are saved by default.
    #[clap(short, long)]
    pub included_chats: Vec<i32>,

    /// Size of batches with messages.
    #[clap(long, default_value = "1000")]
    pub batch_size: i32,

    /// If presented, the previous existing backup will be removed
    #[clap(short, long)]
    pub clean: bool,

    #[clap(subcommand)]
    pub auth: Option<SubCommand>,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Start authentication process
    #[clap(version = "0.1.0", author = "Alex Plate <AlexPl292@gmail.com>")]
    Auth(Auth),
}

#[derive(Clap)]
pub struct Auth {}