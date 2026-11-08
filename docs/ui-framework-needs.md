# What Opium's window needs from a UI framework

Opium's desktop window is drawn with egui and will move to uwui, the owner's UI framework, formerly
slime-ui ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md),
[ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)). uwui's first stage, which
ports Opium, ends only when every need marked *indispensable* below exists in uwui, shown by an
example and its tests (uwui's ROADMAP;
[ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)). This file is that
list.

Each need was found in the code of `crates/app-desktop`; the inventory written for ADR-0170 on
2026-11-04 was not kept, so this one replaces it.
[A later section](#what-adr-0034-asks-of-uwui) adds what the owner's requirements ask beyond
today's window ([ADR-0034](adr/0034-what-opium-is-to-become.md)); its needs are counted
apart from the 113 below. [The last](#what-uwuis-decisions-of-2026-11-07-ask) adds what uwui's
decisions of 2026-11-07 ask
([ADR-0037](adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)).

## How to read it

- **Need**: what the window asks, in a framework's terms; egui's names are in the third column.
- **Asked by**: where the code asks it. Paths are under `crates/app-desktop/src/` unless they name
  a crate; line numbers are those of commit `f368584`.
- **egui today**:
  - *Given*: egui or eframe provides it, and Opium uses it.
  - *Built*: Opium builds it itself on lower pieces (hand-made meshes, its own timers).
  - *Unused*: egui provides it, Opium does not use it yet.
  - *Missing*: neither egui nor Opium has it.
- **Mark**: *Indispensable* when the port would lose a function Opium has, or the look the owner
  approved for one of its built-in themes, or when the [quality bar](ROADMAP.md#quality-bar)
  requires it once the window is on uwui. *Wished* otherwise.

The owner confirmed this rule and the marks on 2026-11-05, and chose four needs as indispensable
that the rule left wished: a Korean fallback and colour emoji, tabular figures, WebP and GIF
covers, and mipmaps.

Of 113 needs, 108 are indispensable and 5 wished. egui gives 70 of them and Opium builds 28 itself,
mostly drawing and motion (gradients, morphing shapes, springs, masks). 6 are in egui unused, and
9 are missing. A row whose third column is mixed ("Given for X; built for Y") is counted by its first
word.

## How the window is built

Three facts shape every row below. A framework with a retained tree, laid out by taffy as uwui
is, must be able to express each of them:

- **Controls are drawn by hand at rectangles Opium computes.** There are 110 `interact` calls and
  84 calls to `allocate_exact_size`, `put` or `painter_at`, against 13 uses of egui's layouts. egui's
  buttons, labels and sliders are not used at all. Opium therefore needs to place an element
  absolutely, hit-test any rectangle, and give it an id that stays stable across frames.
- **State lives between frames, keyed by id.** Springs, entrances, scroll positions and the
  carousel's focus are kept that way, in 46 uses of `ui.data`. A retained framework keeps this
  state on its elements.
- **The theme picks components, not only colours** ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)).
  A player bar, for example, can be floating, docked or a toolbar.

## What stays in Opium

uwui takes an application's need only if it is general (ADR-0170). These are Opium's own work.
They stay in Opium and ask the framework only for what the rows below list:

- The media controls (MPRIS) and the single instance, over D-Bus on threads of their own
  (`desktop/src/mpris.rs`, `main.rs:21`).
- The file pickers through the desktop portal (`app.rs:295`, `app.rs:1353`), and the keyring
  (`downloads`).
- Fetching covers, signed for Navidrome, with their disk cache (`covers.rs`, `desktop/src/covers.rs`);
  the colours taken from a cover (`opium-core/src/palette.rs`).
- Decoding animated covers with FFmpeg (`crates/motion`); the framework only shows the frames.
- The themes as TOML files and the Amane palette, and watching those files: a thread polls them
  each second (`app.rs:6372-6403`), [known debt](debt.md).
- The player's state: the window asks for it every 250 ms while music plays (`app.rs:6245`). The
  quality bar wants a frame only when the progress moves a pixel. The player waking the window is
  [known debt](debt.md).

## The window

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| One window with a title, an application id (`opium`), an icon, a starting size and a minimum size (760 × 520) | `main.rs:30-41` | Given: `ViewportBuilder` | Indispensable |
| The system's title bar and decorations; no client-side decorations, no transparent window | `main.rs:31` | Given | Indispensable |
| Going fullscreen and back, and knowing which it is | `ui/now_playing.rs:156-169`, `:708-744` | Given: `ViewportCommand::Fullscreen`, `viewport().fullscreen` | Indispensable |
| Bringing the window to the front when asked from another thread (a second launch, MPRIS Raise) | `app.rs:5726` | Given: `ViewportCommand::Focus` | Indispensable |
| Closing the window from code (MPRIS Quit) | `app.rs:5727` | Given: `ViewportCommand::Close` | Indispensable |
| A hook when the window closes, before teardown, to save the session | `app.rs:6146` | Given: `App::on_exit` | Indispensable |
| A teardown that never hangs | `desktop/src/exit.rs:21-34` | Built: a watch aborts the process 5 s after the session is saved, as eframe's teardown sometimes hangs ([ADR-0021](adr/0021-the-exit-watch.md), [ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md)) | Indispensable |
| The interface scaled by the user's zoom (0.7 to 2.5, times a base of 1.1) on top of the display's fractional scale, with the keys chosen by Opium | `app.rs:849`, `app.rs:6122-6141` | Given: `set_zoom_factor`, `zoom_with_keyboard` off | Indispensable |
| The display's scale readable while drawing, and followed when it changes (the window moved to another display), so hand-made shapes and images are sized in physical pixels | `ui/shapes.rs:82`, `:92`, `ui/mod.rs:745`, `ui/thumbs.rs:31` | Given: `pixels_per_point`, read at each frame | Indispensable |

## Waking and frames

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Woken from any thread: a function, `Send + Sync`, that asks for a frame (`desktop::Wake`) | `desktop/src/lib.rs:20`, `app.rs:336-339`, `covers.rs:123`, `ui/thumbs.rs:101`, `fonts.rs:69` | Given: `request_repaint` from a cloned `Context` | Indispensable |
| A frame asked for after a delay, nothing drawn until then: toasts, debounce, retries | `ui/mod.rs:848`, `app.rs:6207`, `ui/settings.rs:725` | Given: `request_repaint_after` | Indispensable |
| Nothing drawn at rest | the whole window; [measured](measurements.md#1-at-rest-cpu-time-and-wake-ups) | Given: eframe draws only when asked | Indispensable |
| The application's logic run every time something wakes it, even when the window is not drawn (on another workspace), so that MPRIS, the queue and devices keep going | `app.rs:6176` | Given: `App::logic` apart from `App::ui` | Indispensable |
| A frame rate capped per animation | `ui/lyrics.rs:280-287` (30 a second while a word fills, 10 between words), `ui/loading.rs:25` (the placeholder at 30), `:51` (the spinner at 20), `ui/tracks.rs:697` (the equaliser at 20), `app.rs:3662` (animated covers at their own rate) | Built: each animation asks for its next frame after a delay. Springs, entrances, the M3E loader and the turning cookie are not capped and draw at the display's rate (`ui/motion.rs:145`, `ui/loading.rs:81`, `ui/fab.rs:32`) | Indispensable |
| Whether the window has the focus, so decorative animations stop without it | `ui/fab.rs:29`, `ui/toolbar.rs:364`, `ui/tracks.rs:674` | Given: `input.focused`. The lyrics and the loaders still run without the focus ([known debt](debt.md)) | Indispensable |
| Whether the window is seen at all (minimised or covered), so animated covers stop and free their decoders | `app.rs:6235-6238` | Unused: `ViewportInfo::minimized`, `occluded`; Opium guesses from 2 s without a frame | Indispensable: the quality bar's "playing, window hidden" |
| The time and the frame's step, for animations | `ui/grid.rs:426`, `app.rs:6186` | Given: `input.time`, `stable_dt` | Indispensable |

## Driven and measured

The quality bar measures frame times, input to response and the look through uwui. Under egui,
M1.14 measured frame times with a record and scripted scenarios of the window's own
(`measure.rs`), removed with egui; [its figures](measurements.md#the-figures-to-beat-on-uwui-m114-2026-11-07)
are the ones uwui must match, measured the same way: the same library, scenarios, speed and
display.

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| A frame read back as pixels, for the screenshot mode (`OPIUM_SCREENSHOT`) and reference images | `screenshot.rs:137-157` | Given: `request_screenshot` | Indispensable |
| The window rendered without a display, for reference images in CI and captures off screen | the quality bar's *Rendering* and *The look in one place* | Missing: the window needs a compositor (a headless Weston for tests) | Indispensable |
| Agents reading the interface and driving it: scroll, click, type | the quality bar's *Input to visible response*; [known debt](debt.md) | Missing, but for M1.14's scripted scroll and transitions, injected into egui's input (`App::raw_input_hook`) | Indispensable |
| Each frame's time, recorded, per animation | the quality bar's *Scrolling and animations* | Given for M1.14: each frame's start, eframe's CPU time for it (`IntegrationInfo::cpu_usage`) and whether the frame before asked for it at once (`requested_repaint_last_pass`); no GPU time | Indispensable |

## Input

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Keys with modifiers, repeated while held, consumed so that one handler gets them: Space, Escape, Ctrl+arrows (held for the volume), Shift+arrows (seek), Ctrl+M, S, R, L, J, the zoom keys | `ui/mod.rs:918-1035`, `app.rs:6123-6129` | Given: `key_pressed`, `consume_key`, both counting repeats | Indispensable |
| Ctrl+F opens the page's search | `ui/mod.rs:1064` | Given | Indispensable |
| Knowing whether a text field or a dialog holds the keyboard, so shortcuts stand aside | `ui/mod.rs:914-917` | Given: `egui_wants_keyboard_input`, `top_modal_layer` | Indispensable |
| Hit-testing a rectangle Opium computes, with an id stable across frames: click, double click (plays a song), right click | `ui/tracks.rs:642-651`, 110 `interact` calls | Given: `Ui::interact`, `Sense` | Indispensable |
| Whether the pointer is held down on a control, for M3E's press morph | `ui/fab.rs:123`, 12 uses in all | Given: `is_pointer_button_down_on` | Indispensable |
| Drag with the pointer's position, and its end: the seek and volume bars, the time previewed during the drag | `ui/dock.rs:394-462`, `ui/player_bar.rs:443-470`, `ui/now_playing.rs:680-698` | Given: `Sense::click_and_drag`, `drag_stopped`, `interact_pointer_pos` | Indispensable |
| Hover, and how long the pointer has rested: card lifts, a song fetched ahead once rested on, Now Playing's controls hidden when the pointer is still | `ui/grid.rs:81`, `ui/mod.rs:1186-1199`, `ui/now_playing.rs:78-91` | Given: `hovered`, `time_since_last_movement` | Indispensable |
| The mouse's back and forward buttons | `ui/mod.rs:938-951` | Given: `PointerButton::Extra1`, `Extra2` | Indispensable |
| Smooth scrolling, vertical and sideways, read as a delta by lists that scroll themselves (shelves, lyrics) | `ui/grid.rs:422`, `ui/lyrics.rs:199` | Given: `smooth_scroll_delta` | Indispensable |
| The pointer's shapes: a hand on what can be clicked, a text cursor on fields, none when Now Playing hides its controls | `ui/fab.rs:102`, `ui/search_bar.rs:157`, `ui/now_playing.rs:80-88` | Given: `CursorIcon` | Wished: a refinement |
| Files dropped on the window, as paths | `app.rs:6194-6201` | Given under X11 and Xwayland only: winit 0.30 has no drag and drop on Wayland | Indispensable, on Wayland too |

No row is dragged and dropped, and there is no touch or gamepad input. Space toggles playback again
on each key repeat while held (`ui/mod.rs:922`): [known debt](debt.md).

## Drawing

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Rectangles filled and stroked (inside), each corner its own radius, the radius animated | `ui/pane.rs:112`, `ui/toolbar.rs:595`, `ui/fab.rs:119-170`, `ui/drawer.rs:66` | Given: `rect_filled`, `CornerRadius` (whole points only) | Indispensable |
| Lines, circles and segments: the signal path, the spinner, dots | `ui/signal.rs:385-451`, `ui/loading.rs:11-52`, `ui/lyrics.rs:475` | Given | Indispensable |
| A line of many segments, its points moving each frame: the wavy seek bar of the M3E theme | `ui/toolbar.rs:335-440` | Given: `Shape::line` | Indispensable |
| A convex polygon (the Dolby mark) | `ui/mod.rs:1296-1308` | Given: `Shape::convex_polygon` | Indispensable |
| One or two soft shadows on an element, with offset, blur and spread, following its radius | `ui/toolbar.rs:48-58`, `ui/pane.rs:164-182`, `ui/grid.rs:260` | Given: `Shadow` | Indispensable |
| Linear gradients of two to five stops, some transparent, vertical and diagonal, inside rounded shapes | `ui/detail.rs:784`, `ui/artist.rs:132`, `:1190`, `:1304`, `ui/mod.rs:602-631`, `ui/home.rs:487` | Built: meshes coloured per vertex, mixed in OKLab | Indispensable |
| Radial gradients: elliptical glows behind pages, a colour wash drifting behind Now Playing | `ui/mod.rs:1475-1581`, `ui/now_playing.rs:403-425` | Built: a 24 × 16 grid mesh coloured per vertex | Indispensable |
| Gradients dithered, so long dark fades show no bands | none | Missing | Indispensable: the quality bar's *Rendering* |
| Closed vector shapes that morph into one another: seven M3E shapes of 120 radii each, eased radius by radius, rotated, filled with a colour or an image | `ui/shapes.rs:1-178`, `ui/fab.rs:55-68`, `ui/loading.rs:56-82` | Built: a fan of triangles | Indispensable |
| Transforms of what is drawn: scale about a point (a lyrics line grows), rotation (a cover spins) | `ui/lyrics.rs:253-262`, `ui/shapes.rs:71-78`, `ui/toolbar.rs:178` | Given for scale: `Shape::transform`; rotation built by turning points and texture coordinates | Indispensable |
| The opacity of a group, for fades | `ui/motion.rs:62-64`, `ui/now_playing.rs:533` | Given: `Ui::set_opacity` | Indispensable |
| Clipping to a rectangle | `ui/pane.rs:56`, `ui/grid.rs:571`, `ui/lyrics.rs:214` | Given: `set_clip_rect`, `painter_at` | Indispensable |
| Clipping to a rounded rectangle, and masks that fade content at an edge: the M3E panes, the lyrics' top and bottom, the cover fading into the lyrics | `ui/mod.rs:725-766`, `ui/lyrics.rs:451-460`, `ui/now_playing.rs:471-494` | Built: corners painted over in the page's colour; opacity per line; a mesh with alpha per vertex | Indispensable |
| Layers in a known order: pages, then floating bars, drawers, menus, tooltips, and a veil over the whole window; a layer raised to the top while the window runs; each layer takes the pointer or lets it through | `ui/mod.rs:729-740`, `:770-784`, `ui/mod.rs:331`, `ui/drawer.rs:44-52`, `ui/motion.rs:89` | Given: `LayerId`, `Order`, `Area`, `move_to_top`; some layers registered by hand | Indispensable |
| Translucent fills over content | `ui/now_playing.rs:547` | Given | Indispensable |
| What lies behind an element, blurred, for glass | none | Missing | Wished: no theme uses it yet |
| SVG icons, tinted, drawn at fractional positions: 70 icons, some logos with gradients of their own | `icons.rs`, `icons.rs:213-216`, `ui/mod.rs:1229-1232` | Given: `egui_extras`' SVG loader (resvg), `Image::tint`, `paint_at` | Indispensable |
| Edges anti-aliased on every shape, its own and Opium's, and sharp at scales 1, 1.5 and 2 | `ui/shapes.rs:82`, `:115`, `ui/mod.rs:744` | Given for egui's shapes; Opium's meshes feathered by hand, one physical pixel wide | Indispensable |

## Images

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Images from files and from HTTPS, through a loader of Opium's own (requests signed for Navidrome) | `covers.rs:67-148` | Given: a custom `BytesLoader` | Indispensable |
| An image asked at the size it is drawn, in physical pixels | `ui/shapes.rs:89-95`, `ui/home.rs:566` | Given: `SizeHint::Size` | Indispensable |
| Images decoded away from the interface's thread, at that size (64 to 1024 px, and 1500 or 3000 px for Now Playing), by a bounded pool of threads | `ui/thumbs.rs:22`, `:88-156`, `ui/now_playing.rs:462-466` | Built: Opium's `ImageLoader` resizes; one thread per cover file, unbounded ([known debt](debt.md)) | Indispensable |
| A cache bounded by Opium (60 s unseen, 400 covers at most) that frees the GPU's copy, keeping no decoded copy once uploaded | `covers.rs:14-54`, `app.rs:852` | Given: `forget_image`, `reduce_texture_memory` | Indispensable |
| An image with rounded corners, or cropped to cover its box with the focus above centre (faces) | `ui/mod.rs:1378-1390`, `ui/home.rs:586-594`, `ui/artist.rs:1170-1188` | Given for corners; cropping built from texture coordinates | Indispensable |
| An image inside a vector shape (artist photos in M3E shapes) | `ui/shapes.rs:89-113` | Built: a textured mesh | Indispensable |
| Two images cross-faded when the song changes | `ui/mod.rs:1394-1431` | Built: two images, one tinted transparent | Indispensable |
| A texture updated in place at each frame, for animated covers at 25 frames a second | `app.rs:3612-3690` | Given: `TextureHandle::set_partial` | Indispensable |
| Knowing whether a rectangle is on screen, so an animated cover out of sight stops | `ui/mod.rs:1445`, `ui/artist.rs:1148` | Given: `is_rect_visible` | Indispensable |
| JPEG and PNG | `ui/thumbs.rs:144`, `crates/app-desktop/Cargo.toml` (`image`'s features) | Given through the `image` crate | Indispensable |
| WebP and GIF covers | [known debt](debt.md) | Unused: `image` is built with JPEG and PNG only; decoding them is Opium's, showing them the framework's | Indispensable: the owner's choice |
| Mipmaps, for smooth downscaling | `ui/home.rs:574` filters linearly | Unused: `TextureOptions::mipmap_mode` | Indispensable: the owner's choice |

## Motion

Opium builds most of its motion itself. These are the curves and behaviours its themes use, each
theme with its own durations (`ui/motion.rs:25-30`).

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| A value moved between two states over a time, then eased: hover fades (0.1 to 0.2 s), the drawer sliding (0.24 s), the top bar turning solid, a lyrics line lit | `ui/tracks.rs:389`, `ui/drawer.rs:31-36`, `ui/mod.rs:436-438`, `ui/lyrics.rs:227` | Given: `animate_bool_with_time`, eased by Opium; once with egui's easing (`animate_bool_with_time_and_easing`) | Indispensable |
| Cubic Bézier curves as CSS writes them, `cubic-bezier(0.3, 0, 0.1, 1)` | `ui/lyrics.rs:22-87` | Built: solved by bisection | Indispensable |
| Damped springs that overshoot, M3E's fast and slow (stiffness 800 and 380, damping 0.6 and 0.8), on widths, radii, scales and shapes | `ui/motion.rs:98-149`, `ui/fab.rs:55-68`, `ui/home.rs:436-462` | Built: integrated in 240 Hz steps | Indispensable |
| Delays in cascade: each of the next six lyrics lines follows 45 ms after the one before | `ui/lyrics.rs:26-27`, `:52-65`, `:184-217` | Built | Indispensable |
| Animations driven by scrolling: the top bar's fill, the banner bar shown past the banner | `ui/mod.rs:429-444`, `ui/detail.rs:1160-1170` | Built: read from the scroll offset | Indispensable |
| An element's entrance: a page fades in, and rises in Apple's and M3E's themes (14 px over 0.38 s for M3E); Now Playing rises 48 px | `ui/motion.rs:10-67`, `ui/mod.rs:86-87`, `:280-281` | Built | Indispensable |
| A cross-fade of the whole window when the theme changes | `ui/motion.rs:71-115` | Built: a veil in the old colour on the top layer | Wished: a refinement |
| A busy indicator while a song loads, sized and tinted by the theme | `ui/fab.rs:86-88`, `ui/dock.rs:323`, `ui/player_bar.rs:120` | Given: `Spinner` | Indispensable |
| Animations that loop: the turning cookie (14 s), the loader morphing through six shapes, the equaliser bars, three breathing dots | `ui/fab.rs:19-36`, `ui/loading.rs:11-82`, `ui/tracks.rs:672-700`, `ui/lyrics.rs:462-477` | Built | Indispensable |
| Reduced motion and reduced transparency, from the system or a setting | none | Missing | Indispensable: the quality bar's *Accessibility* |

## Lists and layout

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| An element placed at a rectangle Opium computes, painted by Opium | 84 calls to `allocate_exact_size`, `put` or `painter_at` | Given | Indispensable |
| State kept between frames per element, by id: springs, entrances, scroll targets, focus | 46 uses of `ui.data` | Given: `Context::data` | Indispensable |
| Vertical lists of thousands of rows where only the rows in view are built | `ui/rail.rs:184`, `ui/library.rs:254`, `ui/tracks.rs:712`, `ui/songs.rs:39`, `ui/detail.rs:933` | Given: `ScrollArea::show_rows`, `show_viewport`; the visible range computed by Opium | Indispensable |
| A grid of covers, its columns from the width, only the rows in view built | `ui/grid.rs:622`, `:811` | Given: `show_viewport`; layout built | Indispensable |
| Scroll bars styled by the theme, shown only when needed | `ui/rail.rs:183`, `theme.rs:354-375` | Given: `ScrollBarVisibility::VisibleWhenNeeded`, `Visuals` | Indispensable |
| Shelves that scroll sideways, eased, with arrows, cards out of view skipped | `ui/grid.rs:395-560`, `ui/home.rs:614`, `:726` | Built over a clipped child | Indispensable |
| Children laid out in a row or a column, centred on the other axis, inside margins; some from right to left | `ui/search_bar.rs:57`, `ui/window_bar.rs:25`, `ui/toolbar.rs:250`, `:446`, `ui/np_expressive.rs:217`, `ui/player_bar.rs:73`, `:212`, `ui/dock.rs:478` | Given: `Layout`, `Frame` with `Margin` | Indispensable |
| A carousel with a focused item that grows | `ui/home.rs:388-557` | Built | Indispensable |
| A table's header kept on top once its rows scroll past (Spotify's bar and column titles) | `ui/detail.rs:1160-1190`, `ui/mod.rs:665-697` | Built: drawn again on a layer from the scroll offset | Indispensable |
| The scroll position kept per page (the page, the artist, the query), and a row scrolled into view | `ui/detail.rs:931`, `ui/artist.rs:414`, `ui/search.rs:135`, `ui/tracks.rs:749-754` | Given: `id_salt`, `scroll_to_rect` | Indispensable |
| Fixed panels round a central page (top bar, sidebar or rail, side panel, dock), one width animated by a spring | `ui/mod.rs:136`, `:148-250` | Given: `Panel`, not resizable | Indispensable |
| A layout that changes under a width: below 1100 px the sidebar folds, the side panel becomes a drawer and the padding tightens | `ui/mod.rs:61`, `:110-111`, `:210`, `:298` | Built from the available width | Indispensable |
| Menus: a context menu on right click, the same menu from a button, anchored on the side with room | `ui/menu.rs:36-60`, `:584`, `ui/devices.rs:60-78` | Given: `Popup::context_menu`, `Popup::menu`, `RectAlign` | Indispensable |
| Modal dialogs over a backdrop, inside the window, closed by Escape or a click outside | `ui/playlist_dialog.rs:53`, `:311`, `:564`, `:645`, `ui/info_dialog.rs:55`, `ui/pairing.rs:30`, `:111` | Given: `Modal`, `should_close` | Indispensable |
| A popover anchored above or below a button (the signal path) | `ui/signal.rs:335-348` | Built on the tooltip layer | Indispensable |
| Tooltips on icon buttons | `ui/search_bar.rs:70`, `ui/dock.rs:123` | Given: `on_hover_text` | Indispensable |
| Toasts that leave after 5 s | `ui/mod.rs:838-905` | Built | Indispensable |

## Text

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Fonts by weight: Geist at 400, 500, 600 and 700 | `fonts.rs:1-56`, `:91-109` | Built: egui has no weights, so each is a family of its own | Indispensable |
| Fallback fonts per script from the system (Greek, Japanese, which also covers Chinese, Arabic, Hebrew, Thai, Devanagari), added after start-up on a thread, kept upright, variable ones set to the weight asked | `fonts.rs:58-72`, `:99-103`, `:118-182` | Built on `set_fonts` and fontconfig; `FontData`'s axis coordinates given | Indispensable |
| A Korean fallback, and colour emoji | `fonts.rs:121`, `:85` | Missing: monochrome emoji only ([known debt](debt.md)) | Indispensable: the owner's choice |
| Text styled by ranges: in one layout, runs of their own colour and opacity | `ui/lyrics.rs:374-435` | Given: `LayoutJob` sections | Indispensable |
| Lyrics lit word by word: a soft wipe across the letters of the word being sung, laid out each frame, cached when unchanged | `ui/lyrics.rs:346-435` | Built on `LayoutJob`; the cache is egui's | Indispensable |
| One line cut with "…" at a width; at most two lines then "…"; wrapping at a width | `ui/mod.rs:1204-1208`, `ui/home.rs:806-822`, `ui/lyrics.rs:433`, `:448` | Given: `TextWrapping` | Indispensable |
| Letter spacing and line height set per style | `ui/lyrics.rs:387-388` | Given: `extra_letter_spacing`, `line_height` | Indispensable |
| Text laid out, measured, then drawn where Opium places it | `ui/mod.rs:1211-1227`, `ui/search_bar.rs:196-199` | Given: galleys | Indispensable |
| Tabular figures, so the time does not shift each second | `ui/player_bar.rs:175-186` | Built: room measured with every digit a 0; egui has no OpenType features | Indispensable: the owner's choice |
| Text fields of one line, without a frame, with a hint, vertically centred, the theme's cursor and selection colours, focus asked and given up, Enter submits | `ui/search_bar.rs:203-222`, `ui/window_bar.rs:162-170`, `ui/mod.rs:1140-1157`, `ui/sidebar.rs:157-169`, `theme.rs:366-374` | Given: `TextEdit`; centred by sizing it one line tall | Indispensable |
| A text field of several lines (a playlist's notes) | `ui/playlist_dialog.rs:463-468` | Given: `TextEdit::multiline` | Indispensable |
| A masked field (the Navidrome password) | `ui/settings.rs:816-821`, `ui/playlist_dialog.rs:675-676` | Given: `TextEdit::password` | Indispensable |
| A field limited in length (the six-digit pairing code), centred | `ui/pairing.rs:153-167` | Given: `char_limit`, `align` | Indispensable |
| Copying and pasting in fields, and copying to the clipboard from code | `app.rs:6189`, `ui/window_bar.rs:162` | Given: `copy_text`, `TextEdit` | Indispensable |
| Input methods in fields, for Japanese or Chinese searches | `ui/window_bar.rs:162` and every field | Given: `TextEdit` takes IME input | Indispensable |
| Right-to-left text laid out right to left | none | Missing | Wished: Opium is to be translated, English and French first ([ADR-0034](adr/0034-what-opium-is-to-become.md)); this becomes indispensable once a right-to-left language is planned |
| Drawn text selected and copied | none | Unused: `Label::selectable`, but Opium paints its text by hand | Wished: nothing asks it |

## The system

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Opening an address in the user's browser | `ui/artist.rs:537`, `:925`, `ui/detail.rs:1846` | Given: `open_url` | Indispensable |

## Accessibility

Opium gives screen readers nothing today: eframe enables AccessKit, but none of Opium's controls,
drawn by hand, names itself; only the text fields are egui widgets. The quality bar asks for all of
it ([Accessibility](ROADMAP.md#quality-bar)).

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Each control named to screen readers, with its role and state | every `ui.interact` | Unused: `Response::widget_info`, AccessKit | Indispensable: the quality bar |
| A focus ring, and Tab moving between controls | `ui/window_bar.rs:121-142` (the search field only) | Unused: controls that sense clicks can take the focus (`Sense::click` includes `FOCUSABLE`), but Opium draws no ring on them; Tab's order across Opium's layout not checked | Indispensable: the quality bar |
| Keyboard navigation in the song tables: arrows move a selection, Enter plays | `ui/tracks.rs:718-754` | Built | Indispensable |
| Keyboard navigation in the other lists and grids | none | Missing | Indispensable: the quality bar |

## Themes

| Need | Asked by | egui today | Mark |
|---|---|---|---|
| Components built differently per theme, not only coloured differently: three window layouts, three sidebars, three player bars, two side panels | `theme.rs:20-77`, `opium-core/src/theme.rs` | Built: each component branches on its variant | Indispensable |
| The theme changed while the window runs, light or dark, the framework's own widgets (scroll bars, fields, selection) recoloured with it | `theme.rs:354-375`, `app.rs:6178-6188`, `opium-core/src/theme.rs:155` | Given: `set_visuals`, `Visuals::light` and `dark` | Indispensable |

## What uwui must hold

The figures the window reaches with egui today, and the budgets the owner set for uwui, are in the
[quality bar](ROADMAP.md#quality-bar) and [`measurements.md`](measurements.md). Nothing is drawn at
rest. The display's rate holds while scrolling and animating, on the high tier (4K at 240 Hz) and on
the lowest (the owner's very modest laptop, an HP Laptop 17-ca2xxx with an Athlon Silver 3050U, the
one uwui measures first; [ADR-0034](adr/0034-what-opium-is-to-become.md)). The bar also sets the
start-up time and the memory at rest and while scrolling the whole library.

## What ADR-0034 asks of uwui

The rows above come from today's window. The owner's requirements of 2026-11-06
([ADR-0034](adr/0034-what-opium-is-to-become.md)) ask more of uwui: a few for the move itself, the
rest for Opium's later stages. uwui takes an application's need only if it is general (SlimeEngine's
ADR-0170), so the needs are split in two. The third column says whether uwui's own requirements
(SlimeEngine's ADR-0190 and ADR-0199, uwui's ROADMAP) hold the need already, as read on 2026-11-06;
*precision* means they hold it in general terms and Opium says what it needs exactly.

### Opium's state, for M6d.1b3b

SlimeEngine's M6d.1b3b chose how an application's state changes its interface: signals, checked
against this list among others (SlimeEngine's ADR-0205). These are the changes Opium makes, from the
most frequent to the rarest. Each must leave the window's thread free and draw nothing once it is
over.

- **Every frame while something moves**: the progress (the wave), the word filling in the lyrics,
  springs, a gesture's drag. Only a few drawn properties change; no view of the page should be
  built again for them.
- **Every second**: the time played.
- **In bursts, from other threads**: search results arriving service by service, within 100 ms to
  2 s; a library sync changing thousands of rows at once; covers decoded while the user scrolls;
  a download's progress.
- **From another device**: a paired device editing the queue or the history, playback handed over
  to this device.
- **By the user**: a sort, a filter or a selection over 100,000 rows; a song dragged in the queue.
- **Rarely**: the theme or the language changed, which touches every element.

In each case the screen changes in place, animated, the item the user looks at and the selection
kept: an insertion above the visible rows does not move them. SlimeEngine's ADR-0202 scripts Opium's
table (scroll, the wave, a sort, the selection, the theme), which covers the frequent and the user's
changes; it does not yet insert or change rows from another thread while the table is scrolled, nor
keep its anchor and selection then. That case is the one most likely to separate the four ways of
changing state that M6d.1b compares: rebuilding by keys (today's, the control), signals, views that
say they changed, and messages.

### General needs

The mark follows this file's rule, against the quality bar of 0.1.0, as the owner chose on
2026-11-06: of these 20 needs, 5 are indispensable to the end of uwui's first stage; the others are
wished,
each with the stage of Opium's that asks it where there is one.

| Need | Opium's use | In uwui's requirements | Mark |
|---|---|---|---|
| A list or table of 100,000 rows changed in place: rows inserted, removed, moved and updated by key, at the cost of the change and not of the list, animated, the visible item and the selection kept by identity | A library syncing, search results, the queue edited from another device | Precision: ADR-0190 has a table of millions of rows, not its live changes | Indispensable: the quality bar's 100,000 songs |
| State changed from other threads and tasks, in bursts, gathered into the next frame, the window's thread never waiting | Every service, Connect, the covers, the downloads | Precision: ADR-0190's two executors; the state model decides how | Indispensable: with the row above |
| Values that change every frame bound to drawn properties alone, without building views again | The wave, the lyrics, springs | Precision: the state model | Wished |
| A text range's style animated over time without laying the text out again | A word filling as it is sung | New | Wished |
| The window's state restored at start: the navigation's stack, each list's scroll by the item shown rather than by pixels, the selection, the panels, the size | Opium returns where the user was, its library changed meanwhile | Precision: ADR-0190 restores size, place, panels, tabs and scroll | Wished: Stage 6 |
| An element flying from one screen to the next; animations driven by a finger or the touchpad, interrupted without a jump; predictive back | The cover from the grid to the album to the player | Yes: ADR-0190's motion | Wished: Stage 5 for the song playing, uwui's fifth screen ([ADR-0037](adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)); Stage 6 elsewhere |
| Glass: the application's own content blurred behind a bar or a panel, lightened on the lowest tier, on battery and under reduced transparency | Stock's song playing, then its player bar and sidebar | Yes: the glass material of uwui's core (SlimeEngine's ADR-0171), which Stock uses without uwui's Glass theme ([ADR-0037](adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)); effects lightened | Wished: Stage 5 for the song playing, uwui's fifth screen; Stage 6 elsewhere |
| Haptics on Android | The volume's steps, a list's end | Yes: uwui's ROADMAP | Wished: Stage 6 |
| uwui's high-contrast theme applied to an application whose own themes build components differently, those components staying readable | One high-contrast theme for Opium's three | Precision: ADR-0190 ships a high-contrast theme | Indispensable: the quality bar's accessibility |
| Translation through Fluent, the language changed live; layouts that hold longer text | English and French at 0.1.0 | Yes: ADR-0190 | Indispensable: the quality bar's translation |
| WCAG 2.2 AA checked; every shortcut remapped by the user, a single-key one also turned off | The whole app | Yes: ADR-0190 | Indispensable: the quality bar's accessibility |
| Shortcuts with chords; a palette of commands | The whole app | Yes: ADR-0190 | Wished: Stage 6 |
| A second, small window kept above the others | The mini-player | New: Wayland has no common protocol for it; where the compositor refuses, an ordinary window | Wished: Stage 6 |
| An icon in the tray, notifications with an image, a start with the session | The tray, the song changed, the start with the session | Yes: ADR-0190's system around it | Wished: Stage 6 |
| A theme format users write by hand, choosing a component's variant (its structure, not only its colours), reloaded live, its errors reported without a crash | Opium decides at Stage 5 whether users' themes move to uwui's format | Precision: ADR-0190 lets a theme build a component differently | Wished: Stage 5's decision |
| The application updating itself on Windows and macOS | Opium's Stages 4 and 10 | Deferred: ADR-0199 decides the optional crate "when a pilot needs it"; Opium needs it at Stage 4 | Wished: Stage 4 |
| The lowest Android version uwui supports, stated early | Opium decides its own at Stage 3, as low as uwui allows, aiming at Android 10 or 12 | New: uwui names its lowest phone (Android 14), not its lowest Android | Wished: Stage 3 |
| Charts | The statistics and the year in review | Yes: ADR-0190's optional crate | Wished: Stage 7 |
| A canvas of paths and shapes | The signal path drawn as a chain | Yes: ADR-0190 | Wished: Stage 6 |
| A desktop interface and a phone interface over one application's model, each written for its platform | Opium keeps two interfaces | Yes: ADR-0190 allows screens of one platform | Wished |

### Opium's own

These stay in Opium; uwui gives them only the general pieces above.

- Opium Connect: playback resumed and handed over, the guest on the local network, UPnP and DLNA.
- `opium-headless`, the player without a screen: no uwui at all.
- The sound: the levelling, the crossfade, the equaliser and AutoEq's profiles, the signal path's
  data, bit-perfect USB on Android (in the Kotlin layer).
- The lyrics' sources, word timings and translations; scrobbling; MusicBrainz; Discord.
- The `opium` command, which runs the actions Opium declares on uwui, over the session bus on
  Linux, another way on Windows (Stage 4); MPRIS until uwui's crate for the system's integrations
  exists (ADR-0190).
- The tokens in the keyring; the downloads and the offline mode; the browser sign-in and the CDM,
  from inside Flatpak's sandbox.
- On Android, the home-screen widget, the quick-settings tile, sharing a link and Android Auto, all
  in the Kotlin layer: the system draws them, not uwui.
- The themes' content (Stock with its glass, the renamed Spotify theme, M3E's parts proper to a
  music player on uwui's M3E crate) and the colours taken from covers.
- The smart playlists' rules and their editor, built from uwui's forms.

## What uwui's decisions of 2026-11-07 ask

SlimeEngine's ADR-0213, ADR-0215, ADR-0218 and ADR-0220, taken into Opium by
[ADR-0037](adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md). They serve SlimeEngine's
M6d.9b, which checks the declaration of actions, and M6d.10, which weighs the pilots' needs for
uwui's first stage. Their marks wait for the owner; none is counted with the needs above.

- **The fifth screen of reference.** Opium's song playing, in Stock, with glass, chosen by the
  owner on 2026-11-07. Opium has no Glass theme: Stock uses the glass material of uwui's core. The
  cover flies to it from the screen before, and the transitions move on springs. Built at Opium's
  Stage 5.
- **M3E as an official theme.** uwui's Material 3 Expressive crate holds the generic look. Opium
  builds on it what is proper to a music player: the player bar, the wave of progress, the
  composition of its pages. The crate must let an application add components of its own in the
  theme's style, and change a component's structure, as SlimeEngine's M6d.1d plans for themes.
- **Actions declared once.** Opium's first list, below, for checking the declaration against
  Android's AppFunctions, Apple's App Intents and Windows' registry of agents. The declaration must
  hold an action confirmed in the window each time it runs, an action whose result is a list, and a
  parameter that names an item of the library by its id or by text that the app resolves.
- **A primary modifier.** Opium's shortcuts, every one remappable, saved once with Ctrl or Cmd as
  the primary modifier (SlimeEngine's M6d.3).
- **Crash reports on the machine,** from uwui's desktop host (SlimeEngine's M6d.7): Opium has no
  panic hook today ([quality bar](ROADMAP.md#quality-bar)).
- **A blank window** measured beside Opium's screens, so that Opium's own cost is known.

| Action | Parameters | Family | Permission |
|---|---|---|---|
| Play an item: a song, an album, an artist, a playlist, a station | the item | Playback | Once the agent is accepted |
| Pause, resume, next, previous, seek | a position in seconds, for seek | Playback | Once the agent is accepted |
| Shuffle, repeat, the sleep timer | on or off; a mode; a duration | Playback | Once the agent is accepted |
| Add to the queue, play next, remove from it, clear it | the item; its place in the queue | Queue | Once the agent is accepted |
| What is playing; the queue | none | Reading | Once the agent is accepted |
| Search the library and the services | text; the kinds of item | Reading | Once the agent is accepted |
| Open a page: an album, an artist, a playlist, a search | the item | Navigation | Once the agent is accepted |
| Like a song or take the like back; add it to the library | the item | A service's account | Confirmed in the window each time |
| Create a playlist at a service, add to it, remove from it | the playlist; the items | A service's account | Confirmed in the window each time |
| Opium's own playlists and favourites changed | the playlist; the items | Opium's shared state | Once the agent is accepted, undone by a change that compensates; deleting a playlist confirmed each time |
| Download an album or a playlist, or remove its download | the item | The device's downloads | Confirmed in the window each time |

No setting is reachable by an agent ([ADR-0038](adr/0038-opium-measured-against-the-market.md)). An
agent makes a bounded number of changes in a row; what an action returns marks the text that comes
from outside (titles, notes) as untrusted; "favourite" means Opium's own.
