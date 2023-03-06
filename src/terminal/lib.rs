use termios::{
    tcsetattr, Termios, BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON, OPOST,
    TCSAFLUSH, VMIN, VTIME,
};

use std::os::unix::io::AsRawFd;

pub fn enable_raw_mode() -> Result<Termios, std::io::Error> {
    let fd = std::io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(fd)?;
    let orig_termios = termios.clone();

    termios.c_iflag &= !(BRKINT | ISTRIP | ICRNL | IXON | INPCK);
    termios.c_oflag &= !(OPOST);
    termios.c_cflag |= CS8;
    termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
    termios.c_cc[VMIN] = 0;
    termios.c_cc[VTIME] = 1;

    tcsetattr(fd, TCSAFLUSH, &termios)?;

    Ok(orig_termios)
}

pub fn restore_terminal(termios: Termios) -> Result<(), std::io::Error> {
    let fd = std::io::stdin().as_raw_fd();
    tcsetattr(fd, TCSAFLUSH, &termios)
}
