use clap::Clap;

#[derive(Clap)]
#[clap(author = "Alex Plate <AlexPl292@gmail.com>")]
pub struct Opts {
    #[clap(short, long)]
    pub included_chats: Vec<i32>,
    #[clap(long, default_value = "1000")]
    pub batch_size: i32,
    #[clap(long, default_value = "false")]
    pub clean: bool,
    #[clap(subcommand)]
    pub auth: Option<SubCommand>,
}

#[derive(Clap)]
pub enum SubCommand {
    #[clap(version = "0.1.0", author = "Alex Plate <AlexPl292@gmail.com>")]
    Auth(Auth),
}

#[derive(Clap)]
pub struct Auth {}
