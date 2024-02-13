use nokhwa::utils::{CameraFormat, CameraIndex, CameraInfo, RequestedFormat, Resolution};
use std::sync;
use std::sync::{
    Arc,
    Mutex,
    mpsc,
};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time;
use nokhwa::Camera;
use log::{debug, error, trace};
use mavlink::{
    common::{GpsFixType, MavMessage, ATTITUDE_DATA, REQUEST_DATA_STREAM_DATA},
    error::{MessageReadError, MessageWriteError},
    read_versioned_msg, write_versioned_msg, MavConnection, MavHeader, MavlinkVersion, Message,
};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use crate::utils::SerialConnection;

mod utils;

struct Data {
    pub pitch: (u64, f32),
    pub image: (u64, image::DynamicImage),
}

fn get_time() -> u64 {
    std::time::Duration::from(nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC).unwrap()).as_micros() as u64
}

fn main() {
    let pitch = sync::Arc::new(sync::Mutex::new((0_u64, 0_f32)));
    let (tx, rx) = mpsc::channel::<Data>();

    let thread_camera = std::thread::spawn({
        let pitch = pitch.clone();
        move || thread_camera(pitch, tx)
    });
    let thread_mavlink = std::thread::spawn({
        let pitch = pitch.clone();
        move || crate::thread_mavlink(pitch)
    });
    let thread_db = std::thread::spawn({
        //let pitch = pitch.clone();
        move || crate::thread_db(rx)
    });

    thread_camera.join().unwrap();
    thread_mavlink.join().unwrap();
    thread_db.join().unwrap();
}

fn thread_camera(pitch: Arc<Mutex<(u64, f32)>>, tx: Sender<Data>) {
    let list = nokhwa::query(nokhwa::utils::ApiBackend::Auto).unwrap();
    let cam_info = list.get(1).unwrap();

    println!("{:?}", cam_info);

    let mutex = sync::Arc::new(sync::Mutex::new(false));

    nokhwa::nokhwa_initialize({
        let mutex = mutex.clone();
        move |status| {
            *(mutex.lock().unwrap()) = true
        }
    });

    while *(mutex.lock().unwrap()) == false {
        sleep(time::Duration::from_secs(1))
    }

    println!("cam init");

    let format = nokhwa::utils::RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(
        nokhwa::utils::RequestedFormatType::Closest(CameraFormat::new(
            Resolution::new(640, 480),
            nokhwa::utils::FrameFormat::YUYV,
            25,
        )),
    );


    let mut cam = nokhwa::Camera::new(cam_info.index().clone(), format).unwrap();
    cam.open_stream().unwrap();
    let _ = cam.refresh_camera_format();
    println!("\nInitialized camera with {:?}", cam.camera_format());

    fn next_frame(camera: &mut Camera) -> image::DynamicImage {
        let frame = camera.frame().map_err(|e| {
            error!("Unable to read next frame: {e}");
            Box::new(e)
        }).unwrap();

        frame
            .decode_image::<nokhwa::pixel_format::RgbFormat>()
            .expect("Image dcoding error")
            .into()
    }

    loop {
        let frame = next_frame(&mut cam);
        //frame.save("image.png").unwrap();
        let p = pitch.lock().unwrap().clone();

        let d = Data {
            pitch: p,
            image: (get_time(), frame),
        };
        tx.send(d).unwrap();
    }
}

fn thread_mavlink(pitch: Arc<Mutex<(u64, f32)>>) {
    let imu = Box::new(SerialConnection::new("/dev/ttyACM0", 921600).unwrap());

    loop {
        match imu.recv_frame() {
            Ok( frame ) => {
                //debug!("successfully obtained IMU frame: {frame:?}");
                match frame.msg {
                    MavMessage::ATTITUDE(atti) => {
                        *(pitch.lock().unwrap()) = (get_time(), atti.pitch);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("Mavlink failure: {e}");
                break;
            }
        };
    }
}

fn thread_db(rx: Receiver<Data>) {
    let mut db = rusqlite::Connection::open("db.sqlite").unwrap();
    db.execute(include_str!("../db.sql"), ()).unwrap();

    while let Ok(data) = rx.recv() {
        let mut image = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new(&mut image);
        data.image.1.write_with_encoder(encoder).unwrap();

        let params: &[&dyn rusqlite::ToSql] = &[
            &data.pitch.0,
            &data.pitch.1,
            &data.image.0,
            &image,
        ];

        let t = db.execute("INSERT INTO main (time_p, pitch, time_i, img) VALUES (?1, ?2, ?3, ?4)", params).unwrap();
    }
}