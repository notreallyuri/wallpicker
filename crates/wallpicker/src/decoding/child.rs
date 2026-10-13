//! Invoked only by the private __decode command, after exec under RLIMIT_AS.
use super::protocol;
use std::io::{self, Read};
use wallpicker_core::media::Source;

pub(super) fn run() -> i32 {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    match serve(&mut input, &mut output) {
        Ok(()) => 0,
        Err(error) => {
            let _ = protocol::write_error(&mut output, error);
            1
        }
    }
}

fn serve(input: &mut impl Read, output: &mut impl io::Write) -> io::Result<()> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: getrlimit writes only to a valid rlimit on our stack.
    if unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut limit) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if limit.rlim_cur == libc::RLIM_INFINITY || limit.rlim_cur != limit.rlim_max {
        return Err(io::Error::other("decoder helper requires a hard address-space limit"));
    }
    let request = protocol::read_request(input)?;
    let source = Source::open(&request.path, request.limits).map_err(io::Error::other)?;
    if source.identity() != &request.identity {
        return Err(io::Error::other("media changed before decoder startup"));
    }
    let mut frames = None;
    loop {
        let mut command = [0];
        match input.read_exact(&mut command) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            result => result?,
        }
        if command[0] != protocol::NEXT {
            return Err(io::Error::other("invalid decoder command"));
        }
        if frames.is_none() {
            frames = Some(source.frames().map_err(io::Error::other)?);
        }
        let mut frame = frames.as_mut().unwrap().next();
        if frame.is_none() {
            // Wallpaper policy ignores finite loop metadata. Fresh decoder state
            // at the boundary resets disposal and never caches an entire loop.
            frames = Some(source.frames().map_err(io::Error::other)?);
            frame = frames.as_mut().unwrap().next();
        }
        let frame = frame
            .ok_or_else(|| io::Error::other("media has no frames"))?
            .map_err(io::Error::other)?;
        protocol::write_frame(output, &frame)?;
    }
}
