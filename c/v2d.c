#include "v2d.h"

static vita2d_pgf *font = NULL;
static pthread_t *font_load_t = NULL;
static SceCtrlData pad = {0};

static void *load_font(UNUSED(void *data)) {
  font = vita2d_load_custom_pgf(PGF_FONT_PATH);
  return NULL;
}

// init vita2d context
void v2d_init() {
  // vita2d
  vita2d_init();
  // set clear screen color
  vita2d_set_clear_color(RGBA8(0x2c, 0x2d, 0x31, 0xFF));
  // Digital buttons + Analog support
  sceCtrlSetSamplingMode(SCE_CTRL_MODE_ANALOG);
  // loadfont
  font_load_t = malloc(sizeof(pthread_t));
  pthread_create(font_load_t, NULL, load_font, NULL);
}

// exit vita2d context
void v2d_exit() {
  /*
   * vita2d_fini() waits until the GPU has finished rendering,
   * then we can free the assets freely.
   */
  vita2d_fini();

  // free font
  if (font != NULL) {
    vita2d_free_pgf(font);
    font = NULL;
  }

  if (font_load_t != NULL) {
    free(font_load_t);
    font_load_t = NULL;
  }
}

void v2d_free_texture(void *data) {
  vita2d_texture *tex = (vita2d_texture *)data;
  vita2d_free_texture(tex);
}

void *v2d_load_png(const char *path) { return vita2d_load_PNG_file(path); }
void *v2d_load_png_buf(const void *buf) { return vita2d_load_PNG_buffer(buf); }
void *v2d_load_jpg(const char *path) { return vita2d_load_JPEG_file(path); }
void *v2d_load_jpg_buf(const void *buf, unsigned long size) {
  return vita2d_load_JPEG_buffer(buf, size);
}

void v2d_draw_texture(const void *texture, float x, float y) {
  vita2d_draw_texture((const vita2d_texture *)texture, x, y);
}

void v2d_draw_texture_scale(const void *texture, float x, float y, float sx,
                            float sy) {
  vita2d_draw_texture_scale((const vita2d_texture *)texture, x, y, sx, sy);
}

void v2d_draw_text(int x, int y, unsigned int color, float scale,
                   const char *text) {
  if (font == NULL) {
    return;
  }
  vita2d_pgf_draw_text(font, x, y, color, scale, text);
}

int v2d_text_width(float scale, const char *text) {
  if (font == NULL) {
    return 0;
  }
  return vita2d_pgf_text_width(font, scale, text);
}

int v2d_text_height(float scale, const char *text) {
  if (font == NULL) {
    return 0;
  }
  return vita2d_pgf_text_height(font, scale, text);
}

unsigned int v2d_ctrl_peek_positive() {
  if (font != NULL && font_load_t != NULL) {
    pthread_join(*font_load_t, NULL);
    free(font_load_t);
    font_load_t = NULL;
  }
  // Get the controller state information (polling, positive logic).
  sceCtrlPeekBufferPositive(0, &pad, 1);
  // Bit mask containing zero or more of ::SceCtrlButtons.
  unsigned int buttons = pad.buttons;
  // tract annlog to left/right/up/down
  int lx = pad.lx;
  int ly = pad.ly;
  if (abs(lx - 128) > 100 || abs(ly - 128) > 100) {
    if (abs(lx - 128) > abs(ly - 128)) {
      buttons = buttons | (lx < 128 ? SCE_CTRL_LEFT : SCE_CTRL_RIGHT);
    } else {
      buttons = buttons | (ly < 128 ? SCE_CTRL_UP : SCE_CTRL_DOWN);
    }
  }
  return buttons;
}

void *v2d_get_full_screenshot() {
  vita2d_texture *copy =
      vita2d_create_empty_texture(VITA_DISPLAY_WIDTH, VITA_DISPLAY_HEIGHT);
  unsigned int *buffer = (unsigned int *)vita2d_get_current_fb();
  int *data = (int *)vita2d_texture_get_datap(copy);
  for (int i = 0; i < VITA_DISPLAY_WIDTH * VITA_DISPLAY_HEIGHT; ++i) {
    data[i] = buffer[i];
  }
  return copy;
}

void *v2d_get_screenshot() {
  vita2d_texture *copy =
      vita2d_create_empty_texture(VITA_DISPLAY_WIDTH, VITA_DISPLAY_HEIGHT);
  unsigned int *buffer = (unsigned int *)vita2d_get_current_fb();
  int *data = (int *)vita2d_texture_get_datap(copy);
  for (int x = 0; x < VITA_DISPLAY_HEIGHT; ++x) {
    if (x % 2 != 0) {
      continue;
    }
    for (int y = 0; y < VITA_DISPLAY_WIDTH; ++y) {
      if (y % 2 != 0) {
        continue;
      }
      int n = VITA_DISPLAY_WIDTH * x + y;
      data[n] = buffer[n];
    }
  }
  return copy;
}

unsigned int v2d_color(int r, int g, int b, int a) { return RGBA8(r, g, b, a); }
