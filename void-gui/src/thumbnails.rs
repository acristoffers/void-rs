/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::{Cursor, Read, Seek, SeekFrom};

use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi;
use libadwaita as adw;

use adw::glib;

use crate::window::VoidWindow;

/// Maximum width or height in pixels for generated thumbnails.
const THUMBNAIL_SIZE: u32 = 128;
/// Metadata key used to cache base64-encoded thumbnails inside the vault store.
const THUMBNAIL_KEY: &str = "_thumbnail_";
/// FFmpeg's `AVSEEK_SIZE` flag, used by the seek callback to report stream length.
const AVSEEK_SIZE: i32 = 0x10000;
/// FFmpeg's `AVERROR_EOF` constant (C macro, not exported by bindgen).
// AVERROR_EOF is a C macro, not exported by bindgen
const AVERROR_EOF: i32 = -(0x20464F45_u32 as i32);

/// Returns `true` if `name` has an image file extension.
fn is_image(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "avif" | "ico"
    )
}

/// Returns `true` if `name` has a video file extension.
fn is_video(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "mp4"
            | "mkv"
            | "avi"
            | "mov"
            | "webm"
            | "wmv"
            | "flv"
            | "m4v"
            | "mpg"
            | "mpeg"
            | "ts"
            | "3gp"
    )
}

/// Decodes an image from `bytes` and returns a JPEG-encoded thumbnail.
fn generate_image_thumbnail(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?;
    let thumb = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    let rgb = image::DynamicImage::ImageRgb8(thumb.to_rgb8());
    let mut buf = Cursor::new(Vec::new());
    rgb.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
    Some(buf.into_inner())
}

// ── FFmpeg in-memory video thumbnail ────────────────────────────────────────

/// Cursor-based in-memory buffer fed to FFmpeg's custom I/O layer.
struct MemoryInput {
    cursor: Cursor<Vec<u8>>,
    size: i64,
}

/// FFmpeg read callback: fills `buf` from the in-memory cursor.
unsafe extern "C" fn read_packet(
    opaque: *mut std::ffi::c_void,
    buf: *mut u8,
    buf_size: i32,
) -> i32 {
    let input = &mut *(opaque as *mut MemoryInput);
    let slice = std::slice::from_raw_parts_mut(buf, buf_size as usize);
    match input.cursor.read(slice) {
        Ok(0) => AVERROR_EOF,
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}

/// FFmpeg seek callback: repositions the in-memory cursor or returns the stream size.
unsafe extern "C" fn seek_packet(opaque: *mut std::ffi::c_void, offset: i64, whence: i32) -> i64 {
    let input = &mut *(opaque as *mut MemoryInput);
    if whence & AVSEEK_SIZE != 0 {
        return input.size;
    }
    let seek = match whence & 0x3 {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return -1,
    };
    input.cursor.seek(seek).map(|p| p as i64).unwrap_or(-1)
}

// RAII wrappers for FFmpeg resources (drop in reverse declaration order)

/// RAII wrapper ensuring the FFmpeg `AVFormatContext` is freed on drop.
struct FmtCtx(*mut ffi::AVFormatContext);
impl Drop for FmtCtx {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::avformat_close_input(&mut self.0);
            }
        }
    }
}

/// RAII wrapper ensuring the FFmpeg `AVIOContext` is freed on drop.
struct AvioCtx(*mut ffi::AVIOContext);
impl Drop for AvioCtx {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::avio_context_free(&mut self.0);
            }
        }
    }
}

/// RAII wrapper ensuring the FFmpeg `AVCodecContext` is freed on drop.
struct CodecCtx(*mut ffi::AVCodecContext);
impl Drop for CodecCtx {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::avcodec_free_context(&mut self.0);
            }
        }
    }
}

/// RAII wrapper ensuring the FFmpeg `AVFrame` is freed on drop.
struct AvFrame(*mut ffi::AVFrame);
impl Drop for AvFrame {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::av_frame_free(&mut self.0);
            }
        }
    }
}

/// RAII wrapper ensuring the FFmpeg `AVPacket` is freed on drop.
struct AvPacket(*mut ffi::AVPacket);
impl Drop for AvPacket {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::av_packet_free(&mut self.0);
            }
        }
    }
}

/// RAII wrapper ensuring the FFmpeg `SwsContext` is freed on drop.
struct SwsCtx(*mut ffi::SwsContext);
impl Drop for SwsCtx {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                ffi::sws_freeContext(self.0);
            }
        }
    }
}

/// Scales `(w, h)` to fit within a `max × max` box while preserving aspect ratio.
fn fit_dimensions(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w <= max && h <= max {
        return (w, h);
    }
    if w >= h {
        (max, (h as f64 * max as f64 / w as f64).round() as u32)
    } else {
        ((w as f64 * max as f64 / h as f64).round() as u32, max)
    }
}

/// Decodes a video frame (seeking to ~1 s) from `data` in memory and returns a JPEG-encoded thumbnail.
fn generate_video_thumbnail(data: Vec<u8>) -> Option<Vec<u8>> {
    ffmpeg::init().ok()?;

    let size = data.len() as i64;
    let mut mem = Box::new(MemoryInput {
        cursor: Cursor::new(data),
        size,
    });

    unsafe {
        // Allocate AVIO context for in-memory reading.
        let buf_size: usize = 32768;
        let avio_buf = ffi::av_malloc(buf_size) as *mut u8;
        if avio_buf.is_null() {
            return None;
        }

        let opaque = &mut *mem as *mut MemoryInput as *mut std::ffi::c_void;
        let avio_ptr = ffi::avio_alloc_context(
            avio_buf,
            buf_size as i32,
            0,
            opaque,
            Some(read_packet),
            None,
            Some(seek_packet),
        );
        if avio_ptr.is_null() {
            ffi::av_free(avio_buf as *mut _);
            return None;
        }
        // _avio must outlive _fmt (declared first → dropped last)
        let _avio = AvioCtx(avio_ptr);

        // Open format context and find the first video stream.
        let fmt_ptr = ffi::avformat_alloc_context();
        if fmt_ptr.is_null() {
            return None;
        }
        (*fmt_ptr).pb = avio_ptr;

        let mut fmt_raw = fmt_ptr;
        if ffi::avformat_open_input(
            &mut fmt_raw,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
        ) < 0
        {
            // avformat_open_input frees the context on failure
            return None;
        }
        let _fmt = FmtCtx(fmt_raw);

        ffi::avformat_find_stream_info(fmt_raw, std::ptr::null_mut());

        // Find the first video stream
        let video_idx = (0..(*fmt_raw).nb_streams as usize).find(|&i| {
            let stream = *(*fmt_raw).streams.add(i);
            (*(*stream).codecpar).codec_type == ffi::AVMediaType::AVMEDIA_TYPE_VIDEO
        })? as i32;

        let stream = *(*fmt_raw).streams.add(video_idx as usize);
        let codecpar = (*stream).codecpar;

        // Open the decoder for the video stream.
        let codec = ffi::avcodec_find_decoder((*codecpar).codec_id);
        if codec.is_null() {
            return None;
        }
        let codec_ptr = ffi::avcodec_alloc_context3(codec);
        if codec_ptr.is_null() {
            return None;
        }
        let _codec = CodecCtx(codec_ptr);
        ffi::avcodec_parameters_to_context(codec_ptr, codecpar);
        if ffi::avcodec_open2(codec_ptr, codec, std::ptr::null_mut()) < 0 {
            return None;
        }

        // Seek to approximately 1 second into the video.
        let tb = (*stream).time_base;
        if tb.den > 0 && tb.num > 0 {
            let target = (tb.den as i64) / (tb.num as i64);
            ffi::av_seek_frame(fmt_raw, video_idx, target, ffi::AVSEEK_FLAG_BACKWARD);
        }

        // Decode one video frame from the stream.
        let pkt_ptr = ffi::av_packet_alloc();
        if pkt_ptr.is_null() {
            return None;
        }
        let _pkt = AvPacket(pkt_ptr);

        let frame_ptr = ffi::av_frame_alloc();
        if frame_ptr.is_null() {
            return None;
        }
        let _frame = AvFrame(frame_ptr);

        let mut decoded = false;
        while ffi::av_read_frame(fmt_raw, pkt_ptr) >= 0 {
            if (*pkt_ptr).stream_index == video_idx {
                if ffi::avcodec_send_packet(codec_ptr, pkt_ptr) >= 0
                    && ffi::avcodec_receive_frame(codec_ptr, frame_ptr) >= 0
                {
                    decoded = true;
                    break;
                }
            }
            ffi::av_packet_unref(pkt_ptr);
        }
        if !decoded {
            return None;
        }

        // Scale the decoded frame to thumbnail size as RGB24.
        let src_w = (*frame_ptr).width as u32;
        let src_h = (*frame_ptr).height as u32;
        if src_w == 0 || src_h == 0 {
            return None;
        }
        let (dst_w, dst_h) = fit_dimensions(src_w, src_h, THUMBNAIL_SIZE);

        let sws_ptr = ffi::sws_getContext(
            src_w as i32,
            src_h as i32,
            std::mem::transmute((*frame_ptr).format),
            dst_w as i32,
            dst_h as i32,
            ffi::AVPixelFormat::AV_PIX_FMT_RGB24,
            ffmpeg::software::scaling::flag::Flags::BILINEAR.bits(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        if sws_ptr.is_null() {
            return None;
        }
        let _sws = SwsCtx(sws_ptr);

        let dst_ptr = ffi::av_frame_alloc();
        if dst_ptr.is_null() {
            return None;
        }
        let _dst = AvFrame(dst_ptr);
        (*dst_ptr).width = dst_w as i32;
        (*dst_ptr).height = dst_h as i32;
        (*dst_ptr).format = ffi::AVPixelFormat::AV_PIX_FMT_RGB24 as i32;
        if ffi::av_frame_get_buffer(dst_ptr, 0) < 0 {
            return None;
        }

        ffi::sws_scale(
            sws_ptr,
            (*frame_ptr).data.as_ptr() as *const *const u8,
            (*frame_ptr).linesize.as_ptr(),
            0,
            src_h as i32,
            (*dst_ptr).data.as_mut_ptr(),
            (*dst_ptr).linesize.as_mut_ptr(),
        );

        // Copy RGB pixel data, accounting for stride padding between rows.
        let stride = (*dst_ptr).linesize[0] as usize;
        let row_bytes = dst_w as usize * 3;
        let mut rgb = Vec::with_capacity(dst_h as usize * row_bytes);
        for y in 0..dst_h as usize {
            let row = std::slice::from_raw_parts((*dst_ptr).data[0].add(y * stride), row_bytes);
            rgb.extend_from_slice(row);
        }

        // Encode the thumbnail as JPEG via the image crate.
        let img = image::RgbImage::from_raw(dst_w, dst_h, rgb)?;
        let dyn_img = image::DynamicImage::ImageRgb8(img);
        let mut buf = Cursor::new(Vec::new());
        dyn_img.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
        Some(buf.into_inner())
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Checks the metadata cache for a previously generated thumbnail.
/// Returns `Some(jpeg_bytes)` on cache hit, `None` on miss.
pub fn cached_thumbnail(window: &VoidWindow, path: &str) -> Option<Vec<u8>> {
    let store_ref = window.store();
    let store = store_ref.as_ref()?;
    let b64 = store.metadata_get(path, THUMBNAIL_KEY).ok()?;
    Some(glib::base64_decode(&b64))
}

/// Returns `true` if the file name looks like it could have a thumbnail.
pub fn supports_thumbnail(name: &str) -> bool {
    is_image(name) || is_video(name)
}

/// CPU-heavy thumbnail generation from raw bytes.  This is `Send`-safe and
/// intended to run on a background thread via `gio::spawn_blocking`.
pub fn generate_thumbnail(name: &str, bytes: Vec<u8>) -> Option<Vec<u8>> {
    if is_image(name) {
        generate_image_thumbnail(&bytes)
    } else if is_video(name) {
        generate_video_thumbnail(bytes)
    } else {
        None
    }
}

/// Writes a generated thumbnail into the store metadata cache without saving.
/// The caller should call `store.save()` after all thumbnails are cached.
pub fn cache_thumbnail(window: &VoidWindow, path: &str, thumb_bytes: &[u8]) {
    let b64 = glib::base64_encode(thumb_bytes);
    let mut store_ref = window.store_mut();
    if let Some(store) = store_ref.as_mut() {
        let _ = store.metadata_set_nosave(path, THUMBNAIL_KEY, &b64);
    }
}
