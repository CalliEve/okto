use rand::{prelude::Rng, seq::SliceRandom};

use crate::models::pictures::{HubbleImageSource, MarsRoverPicture};

pub fn get_date_epic_image(full: &str) -> String {
    full.split(' ')
        .next()
        .expect("no date in EPICImage")
        .replace('-', "/")
}

pub fn get_rover_camera_picture<R>(
    list: Vec<MarsRoverPicture>,
    mut rng: &mut R,
) -> Option<MarsRoverPicture>
where
    R: Rng + ?Sized,
{
    for camera in &[
        "NAVCAM", "PANCAM", "MAST", "FHAZ", "RHAZ", "MAHLI", "CHEMCAM",
    ] {
        let pics = list
            .iter()
            .filter(|p| &p.camera.name == *camera)
            .collect::<Vec<&MarsRoverPicture>>();
        if let Some(pic) = pics.choose(&mut rng) {
            return Some((*pic).clone());
        }
    }
    None
}

pub fn biggest_image_url(src: &HubbleImageSource) -> String {
    src.image_files
        .iter()
        .filter(|i| i.width > 200)
        .next()
        .unwrap_or_else(|| src.image_files.first().expect("No hubble image returned"))
        .file_url
        .as_str()
        .replace("//imgsrc.hubblesite.org/hvi", "https://hubblesite.org")
}
