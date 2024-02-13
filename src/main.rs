use nokhwa::utils::{CameraFormat, CameraIndex, Resolution};
use std::sync;
use std::thread::sleep;
use std::time::Duration;
use nokhwa::Camera;
use log::error;

fn main() {
    let list = nokhwa::query(nokhwa::utils::ApiBackend::Auto).unwrap();
    let cam = list.get(1).unwrap();

    println!("{:?}", cam);

    let mutex = sync::Arc::new(sync::Mutex::new(false));

    nokhwa::nokhwa_initialize({
        let mutex = mutex.clone();
        move |status| {
            *(mutex.lock().unwrap()) = true
        }
    });

    while *(mutex.lock().unwrap()) == false {
        sleep(Duration::from_secs(1))
    }

    println!("cam init");

    let format = nokhwa::utils::RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(
        nokhwa::utils::RequestedFormatType::Closest(CameraFormat::new(
            Resolution::new(640, 480),
            nokhwa::utils::FrameFormat::YUYV,
            25,
        )),
    );

    let mut cam = nokhwa::Camera::new(cam.index().clone(), format).unwrap();
    cam.open_stream().unwrap();
    let _ = cam.refresh_camera_format();
    println!("\nInitialized camera with {:?}", cam.camera_format());

    next_frame(&mut cam);

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

    let frame = next_frame(&mut cam);
    frame.save("image.png").unwrap();
}
