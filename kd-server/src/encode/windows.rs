use ffmpeg_next::codec::encoder::video::Video;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame;
use ffmpeg_next::option::Type::Flags;
use ffmpeg_next::software::scaling;
use crate::capture::{Frame, PixelFormat};
use crate::encode::FrameEncoder;

impl From<PixelFormat> for Pixel
{
    fn from(value: PixelFormat) -> Self {
        match value {
            PixelFormat::Bgra => Pixel::BGRA
        }
    }
}

pub struct FfmpegEncoder
{
    context: Video,
    frame_width: u32,
    frame_height: u32,
    fps: u32,
    scaler: scaling::Context
}

impl FfmpegEncoder
{
    pub fn new(frame_width: u32, frame_height: u32, fps: u32) -> anyhow::Result<Self> {
        ffmpeg_next::init()?;

        let codec = ffmpeg_next::encoder::find_by_name("h264_nvenc").or_else(
            || ffmpeg_next::encoder::find_by_name("libx264")
        ).ok_or_else(|| anyhow::anyhow!("No encoder found for libx264"))?;

        let mut context = ffmpeg_next::codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()?;
        context.set_width(frame_width);
        context.set_height(frame_height);
        context.set_format(ffmpeg_next::format::Pixel::YUV420P);
        context.set_time_base(ffmpeg_next::Rational::new(1, fps as i32));
        context.set_frame_rate(Some(ffmpeg_next::Rational::new(fps as i32, 1)));
        context.set_max_b_frames(0);
        context.set_gop(fps);

        let mut options = ffmpeg_next::Dictionary::new();
        options.set("tune", "ll");
        options.set("repeat_headers", "1");
        options.set("forced-idr", "1");
        let encoder = context
            .open_with(options)?;

        let scaler = scaling::Context::get(
            Pixel::BGRA, frame_width, frame_height,
            Pixel::YUV420P, frame_width, frame_height, scaling::Flags::BILINEAR)?;

        Ok(Self {
            context: encoder.0,
            frame_width,
            frame_height,
            fps,
            scaler
        })
    }
}

impl FrameEncoder for FfmpegEncoder
{
    fn encode_frame(&mut self, frame: Frame) -> anyhow::Result<Vec<Vec<u8>>> {
        // Copy frame data to ffmpeg-compatible object
        let mut ffmpeg_frame = frame::Video::new(Pixel::from(frame.format()), self.frame_width, self.frame_height);
        ffmpeg_frame.data_mut(0).copy_from_slice(frame.data());

        // Make the YUV420P output frame
        let mut yuv_frame = frame::Video::new(Pixel::YUV420P, self.frame_width, self.frame_height);
        self.scaler.run(&ffmpeg_frame, &mut yuv_frame)?;

        // Encode
        self.context.send_frame(&yuv_frame)?;

        // Get packets
        let mut nals = vec![];
        let mut packet = ffmpeg_next::Packet::empty();
        loop {
            match self.context.receive_packet(&mut packet) {
                Ok(_) => {
                    let data = packet.data();
                    match data {
                        Some(annex_b) => {
                            nals.extend(parse_nals_from_annex_b(annex_b));
                        },
                        None => {}
                    }
                }
                Err(ffmpeg_next::Error::Other { errno: EAGAIN }) => break,
                Err(e) => return Err(e.into())
            }
        }

        Ok(nals)
    }
}

fn parse_nals_from_annex_b(annex_b: &[u8]) -> Vec<Vec<u8>>
{
    let mut nals = vec![];
    let mut indices = vec![];

    annex_b.windows(4).enumerate().for_each(|(i, window)| {
        if(window == &[0x00, 0x00, 0x00, 0x01]) {
            indices.push((i, 4));
        }
    });

    annex_b.windows(3).enumerate().for_each(|(i, window)| {
        if window == &[0x00, 0x00, 0x01] {
            if i == 0 || annex_b[i - 1] != 0x00 {
                indices.push((i, 3));
            }
        }
    });

    indices.sort_by(|a, b| a.0.cmp(&b.0));

    // For each pair of indices, compute the nal's boundaries
    for window in indices.windows(2) {
        let start = window[0].0 + window[0].1; // skip start code
        let end = window[1].0;

        nals.push(annex_b[start..end].to_vec());
    }

    // Account for last nal
    let last_idx = indices.last().unwrap();
    nals.push(annex_b[last_idx.0 + last_idx.1..].to_vec());

    nals
}

#[cfg(test)]
mod tests {
    use crate::encode::windows::parse_nals_from_annex_b;

    #[test]
    fn test_parse_nals_from_annex_b() {
        let annex_b = vec![0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x01, 0x1F, 0x2F, 0x3F];
        let nals = parse_nals_from_annex_b(&annex_b);
        assert_eq!(nals[0], vec![0x01, 0x05]);
        assert_eq!(nals[1], vec![0x1F, 0x2F, 0x3F]);
    }
}