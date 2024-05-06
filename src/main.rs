//!
//! You can test this out by running:
//!
//!     cargo run 127.0.0.1:12345
//!
//! And then in another window run:
//!
//!     cargo run ws://127.0.0.1:12345/
//!
//! Type a message into the client window, press enter to send it and
//! see it echoed back.
//! 

use std::env;

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures::StreamExt;

use turbojpeg::{Compressor, Image, PixelFormat};
use base64::prelude::*;
use xiapi::AcquisitionBuffer;

use std::io::Error;
use futures::SinkExt;
use futures::TryStreamExt;

use log::info;


#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    info!("New WebSocket connection: {}", addr);

    let (mut write, mut read) = ws_stream.split();

    let (buffer, mut compressor) = cam_setup();

    // Receive and discard messages from the WebSocket client
    while let Some(read_content) = read.try_next().await.unwrap() {
        if let Ok(text) = read_content.to_text() {
            let trimmed_text = text.trim(); // Trim whitespace
            if trimmed_text  == "one" {
                println!("yo");
                let image_data = get_cam_picture(&buffer, &mut compressor);
                let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(image_data)).await;
            }
        }
    }

}

fn cam_setup() -> (AcquisitionBuffer, Compressor){
    info!("started");
    let mut cam = xiapi::open_device(None).unwrap();
    info!("cam open");

    let mut compressor = Compressor::new().unwrap();
    _ = compressor.set_subsamp(turbojpeg::Subsamp::Gray);
    _ = compressor.set_quality(70);
    cam.set_exposure(10000.0).unwrap();
    
    let buffer = cam.start_acquisition().unwrap();

    (buffer, compressor)
}

fn get_cam_picture(buffer : &AcquisitionBuffer, compressor : &mut Compressor) -> String {

    let image = buffer.next_image::<u8>(None).unwrap();
    let pixel = image.pixel(0, 0);
    match pixel {
        Some(_) => {
            // initialize a Compressor
            let imagec = Image {
                pixels: image.data(), //vec![0; (image.width() as usize) * (image.height()as usize)],
                width: image.width() as usize,
                pitch: image.width() as usize, // there is no padding between rows
                height: image.height() as usize,
                format: PixelFormat::GRAY,
            };

            // compress the Image to a Vec<u8> of JPEG data
            let jpeg_data = compressor.compress_to_vec(imagec.as_deref()).unwrap();
            let jpeg_data_encode = BASE64_STANDARD.encode(jpeg_data);

            jpeg_data_encode
        },
        None => unreachable!("Could not get pixel value from image!"),
    }
}