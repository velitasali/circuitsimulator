#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// 1 when the next begin() would drop the current IOSurfaces.
int cs_vello_shared_reset_needed(void *mtl_device, int width, int height);

// Borrow the IOSurface texture on `mtl_device` for the slot Qt is not sampling.
// Vello renders into this texture. Null on failure.
void *cs_vello_shared_begin(void *mtl_device, int width, int height, int *out_slot, int *out_epoch);

// The GPU has finished writing `slot`. Qt may sample it.
void cs_vello_shared_commit(int slot);

// 1 when a finished frame of this size is already on an IOSurface.
int cs_vello_shared_has_base(int width, int height);

// Copy that frame to the buffer Qt is not sampling, then replace the rectangle
// with `patch_tex`. Returns 1 on success.
int cs_vello_shared_patch(void *patch_tex, int x, int y, int w, int h);

// Render thread. Opens the published IOSurface on Qt's MTLDevice.
// The returned MTLTexture is owned here; the caller must not release it.
void *cs_vello_qt_texture(void *qt_device, int *out_width, int *out_height);

// Wraps that texture as a QSGTexture* for `window` (a QQuickWindow).
// Qt owns the returned texture.
void *cs_vello_wrap_texture(void *mtl_texture, void *window, int width, int height);

uint64_t cs_vello_frame_generation(void);

void cs_vello_shared_release(void);

#ifdef __cplusplus
}
#endif
