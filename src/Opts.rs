use clap::Clap;

#[derive(Clap)]
#[clap(author = "Alex Plate <AlexPl292@gmail.com>")]
pub struct Opts {
    #[clap(short, long)]
    pub included_chats: Vec<i32>,
}
