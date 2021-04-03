# tg_backup

Backup your messages from Telegram messenger.

`v0.1.0` this is a very initial version of the software.

### Installation

At the moment you can install `tg_backup` using Homebrew.

```
brew tap AlexPl292/tg_backup
brew install tg_backup
```

### Usage

- Run `tg_backup auth`. Follow the authentication process.
- Start `tg_backup` to back up your messages.

### Notes

- At the moment only one-to-one chats are backed up
- By default, `tg_backup` creates a session file under `$HOME$/.tg_backup` directory.  
  Use options to modify this behaviour.

### Options

```
USAGE:
    tg_backup [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -c, --clean
            If presented, the previous existing backup will be removed

    -h, --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
        --batch-size <batch-size>
            Size of batches with messages [default: 1000]

    -e, --excluded-chats <excluded-chats>...
            List of chats that are going to be excluded from saving.
            
            If both included-chats and excluded_chats have the same value, the chat will be
            excluded.

    -i, --included-chats <included-chats>...
            List of chats that are going to be saved. All chats are saved by default.
            
            If both included-chats and excluded_chats have the same value, the chat will be
            excluded.

        --session-file <session-file>
            


SUBCOMMANDS:
    auth    
            Start authentication process

    help    
            Prints this message or the help of the given subcommand(s)
```

### License

tg_backup is licensed under the terms of the GNU Public License version 3 or any later version.
