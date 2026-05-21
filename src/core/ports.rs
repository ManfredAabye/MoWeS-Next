use std::io;
use std::net::TcpListener;

pub fn find_free_port(preferred: u16) -> io::Result<u16> {
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", preferred)) {
        let port = listener.local_addr()?.port();
        drop(listener);
        return Ok(port);
    }

    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
