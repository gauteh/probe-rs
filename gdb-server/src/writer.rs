use async_std::{io::Write, net::TcpStream, prelude::*};
use futures::channel::mpsc;
use gdb_protocol::packet::{CheckedPacket, Kind as PacketKind};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;

pub async fn writer(
    packet: CheckedPacket,
    stream: &mut TcpStream,
    packet_stream: &Sender<CheckedPacket>,
    buffer: &mut Vec<u8>,
) -> Result<()> {
    let mut tmp_buf = [0; 128];
    log::debug!("WRITE WIN");

    encode(&packet, stream).await?;
    stream.flush().await?;

    log::debug!("Request ACK for {}", String::from_utf8_lossy(&packet.data));
    'ack: loop {
        log::debug!("Reading");
        let n = stream.read(&mut tmp_buf).await?;
        log::debug!("Done Reading ({})", String::from_utf8_lossy(buffer));
        if n > 0 {
            buffer.extend(&tmp_buf[0..n]);
            // glob.extend(&tmp_buf[0..n]);
            log::info!("Current buf {}", String::from_utf8_lossy(buffer));
        }

        for (i, byte) in buffer.iter().enumerate() {
            match byte {
                b'+' => {
                    log::debug!("Ack received.");
                    buffer.remove(i);
                    break 'ack;
                }
                b'-' => {
                    log::debug!("Nack received. Retrying.");
                    buffer.remove(i);
                    continue 'ack;
                }
                // This should never happen.
                // And if it does, GDB fucked up, so we might as well stop.
                _ => (),
            }
        }
        log::debug!("Done checking ACK");
    }

    super::reader::reader(stream, packet_stream, buffer).await
}

pub async fn encode<W>(packet: &CheckedPacket, w: &mut W) -> Result<()>
where
    W: Write + Unpin,
{
    w.write_all(&[match packet.kind {
        PacketKind::Notification => b'%',
        PacketKind::Packet => b'$',
    }])
    .await?;

    let mut remaining: &[u8] = &packet.data;
    while !remaining.is_empty() {
        let escape1 = memchr::memchr3(b'#', b'$', b'}', remaining);
        let escape2 = memchr::memchr(b'*', remaining);

        let escape = std::cmp::min(
            escape1.unwrap_or_else(|| remaining.len()),
            escape2.unwrap_or_else(|| remaining.len()),
        );

        w.write_all(&remaining[..escape]).await?;
        remaining = &remaining[escape..];

        if let Some(&b) = remaining.first() {
            dbg!(b as char);
            // memchr found a character that needs escaping, so let's do that
            w.write_all(&[b'}', b ^ 0x20]).await?;
            remaining = &remaining[1..];
        }
    }

    w.write_all(&[b'#']).await?;
    w.write_all(&packet.checksum).await?;
    Ok(())
}
