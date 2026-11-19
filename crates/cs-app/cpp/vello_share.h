#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint64_t cs_vello_share_generation(void);

/* Render thread. Wraps the published frame as a QSGTexture* for `window`.
 * Null when zero-copy is not available; the caller uploads pixels instead. */
void *cs_vello_share_adopt(void *window, int *out_w, int *out_h);

#ifdef __cplusplus
}
#endif
