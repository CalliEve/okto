use std::ops::{DerefMut, Range};

use rand::{
    prelude::{Rng, RngCore},
    seq::SliceRandom,
};
use reqwest::Error;

use super::constants::{
    DEFAULT_CLIENT,
    NASA_KEY,
    RNG,
};
use crate::models::pictures::{
    HubbleImageSource,
    MarsRoverPicture,
    MarsRoverPictureRes,
    MarsRoverInformationRes
};

pub fn get_date_epic_image(full: &str) -> String {
    full.split(' ')
        .next()
        .expect("no date in EPICImage")
        .replace('-', "/")
}

async fn fetch_rover_image_from_api(sol: u16, rover: &str) -> Option<Vec<MarsRoverPicture>> {
    Some(
        DEFAULT_CLIENT
            .get(
                format!(
                    "https://api.nasa.gov/mars-photos/api/v1/rovers/{}/photos?sol={}&api_key={}",
                    rover,
                    sol,
                    NASA_KEY.as_str()
                )
                .as_str(),
            )
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json::<MarsRoverPictureRes>()
            .await
            .ok()?
            .photos,
    )
}

pub async fn fetch_rover_camera_picture(rover: &str, sol_range: Range<u16>) -> (MarsRoverPicture, u16) {
    let mut rovers: Vec<MarsRoverPicture> = Vec::new();
    let mut sol: u16 = 0;

    while rovers.is_empty() {
        sol = RNG.lock().await.gen_range(sol_range.clone());

        rovers = fetch_rover_image_from_api(sol, rover).await.unwrap_or(rovers);
    }

    (
        get_rover_camera_picture(rover, &rovers, RNG.lock().await.deref_mut())
            .expect(&format!("No {} picture found", rover)),
        sol,
    )
}

fn get_rover_camera_picture<R>(
    rover: &str,
    list: &[MarsRoverPicture],
    mut rng: &mut R,
) -> Option<MarsRoverPicture>
where
    R: RngCore + ?Sized,
{
    let cams: &[&str] = match rover {
        "spirit" | "opportunity" => &["NAVCAM", "PANCAM", "FHAZ", "RHAZ", "MINITES"],
        "curiosity" => &["MAST", "FHAZ", "NAVCAM", "RHAZ", "MAHLI", "CHEMCAM"],
        "perseverance" => &["MCZ_RIGHT", "MCZ_LEFT", "FRONT_HAZCAM_LEFT_A", "FRONT_HAZCAM_RIGHT_A", "NAVCAM_RIGHT", "NAVCAM_LEFT", "REAR_HAZCAM_LEFT", "REAR_HAZCAM_RIGHT"],
        _ => panic!("unknown rover provided")
    };

    for camera in cams {
        let pics = list
            .iter()
            .filter(|p| p.camera.name == *camera)
            .collect::<Vec<&MarsRoverPicture>>();
        if let Some(pic) = pics.choose(&mut rng) {
            return Some((*pic).clone());
        }
    }
    list.first().cloned()
}

pub async fn get_max_sol(rover: &str) -> Result<u16, Error> {
    Ok(
        DEFAULT_CLIENT
            .get(
                format!(
                    "https://api.nasa.gov/mars-photos/api/v1/rovers/{}?api_key={}",
                    rover,
                    NASA_KEY.as_str()
                )
                .as_str(),
            )
            .send()
            .await?
            .error_for_status()?
            .json::<MarsRoverInformationRes>()
            .await?
            .rover
            .max_sol
    )
}

pub fn biggest_image_url(src: &HubbleImageSource) -> String {
    src.image_files
        .iter()
        .find(|i| i.width > 200)
        .unwrap_or_else(|| src.image_files.first().expect("No hubble image returned"))
        .file_url
        .as_str()
        .replace("//imgsrc.hubblesite.org/hvi", "https://hubblesite.org")
}
