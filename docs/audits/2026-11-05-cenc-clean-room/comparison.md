# Token comparison

Targets:

- `crates/cenc/examples/index_cost.rs`
- `crates/cenc/fuzz/fuzz_targets/index_and_make_clear.rs`
- `crates/cenc/fuzz/fuzz_targets/indexer_prefixes.rs`
- `crates/cenc/fuzz/fuzz_targets/movie_read.rs`
- `crates/cenc/fuzz/fuzz_targets/stream_download.rs`
- `crates/cenc/src/boxes.rs`
- `crates/cenc/src/claims.rs`
- `crates/cenc/src/clear.rs`
- `crates/cenc/src/error.rs`
- `crates/cenc/src/fragment.rs`
- `crates/cenc/src/indexer.rs`
- `crates/cenc/src/lib.rs`
- `crates/cenc/src/movie.rs`
- `crates/cenc/src/stream.rs`
- `crates/cenc/src/table.rs`
- `crates/cenc/src/tests/build.rs`
- `crates/cenc/src/tests/clear.rs`
- `crates/cenc/src/tests/layouts.rs`
- `crates/cenc/src/tests/malformed.rs`
- `crates/cenc/src/tests/mod.rs`
- `crates/cenc/src/tests/mutation.rs`
- `crates/cenc/src/tests/other.rs`
- `crates/cenc/tests/allocation.rs`

Sources:

- kopuz-a58c385: 764 files
- kopuz-1d8b323: 771 files
- fmp4.rs@4522e68: 1 files
- tree: 157 files

## Raw tokens, runs of 50 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 0 | - | 29 |
| kopuz-1d8b323 | 771 | 0 | - | 29 |
| fmp4.rs@4522e68 | 1 | 0 | - | 32 |
| tree | 157 | 1 | 106 | 49 |

### 106 raw tokens: `crates/cenc/src/stream.rs` lines 608 to 619, and tree `crates/downloads/src/sealed.rs` lines 208 to 219

```text
  608 | {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
  610 | let pos = match to {
  611 | SeekFrom :: Start ( p ) => Some ( p ) ,
  612 | SeekFrom :: End ( d ) => self . len . checked_add_signed ( d ) ,
  613 | SeekFrom :: Current ( d ) => self . pos . checked_add_signed ( d ) ,
  614 | } ;
  615 | self . pos =
  616 | pos . ok_or_else ( || io :: Error :: new ( io :: ErrorKind :: InvalidInput , "seek before start" ) ) ? ;
  617 | Ok ( self . pos )
  618 | }
  619 | }
```


## Normalised tokens, runs of 30 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 157 | 64 | 29 |
| kopuz-1d8b323 | 771 | 156 | 67 | 29 |
| fmp4.rs@4522e68 | 1 | 14 | 46 | 29 |
| tree | 157 | 108 | 106 | 29 |

### 64 normalised tokens: `crates/cenc/src/tests/build.rs` lines 376 to 389, and kopuz-a58c385 `daemon/server/src/lyrics/request.rs` lines 3 to 18

```text
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize ,
  386 | pub size : usize ,
  387 | pub duration : u32 ,
  388 | pub fragment : usize ,
  389 | pub entry : u32 ,
```


### 60 normalised tokens: `crates/cenc/src/tests/build.rs` lines 304 to 315, and kopuz-a58c385 `shared/api/src/catalog.rs` lines 81 to 94

```text
  304 | Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux
```


### 59 normalised tokens: `crates/cenc/src/table.rs` lines 95 to 103, and kopuz-a58c385 `daemon/player/src/engine/rt.rs` lines 122 to 131

```text
   95 | struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec < ( u32 , u32 ) > ,
  102 | descriptions : Vec < Description > ,
  103 | fragments : Vec
```


### 59 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 16 to 26, and kopuz-a58c385 `daemon/server/src/ytmusic/browse.rs` lines 305 to 315

```text
   16 | ) -> Expected {
   17 | Expected {
   18 | offset : sample . offset ,
   19 | size : sample . size ,
   20 | duration : sample . duration ,
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples
```


### 56 normalised tokens: `crates/cenc/src/fragment.rs` lines 104 to 115, and kopuz-a58c385 `daemon/server/src/ytmusic/browse/items.rs` lines 121 to 132

```text
  104 | Default ) ]
  105 | struct TrafBoxes {
  106 | tfhd : Option < Header > ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option < Header > ,
  114 | groups : Vec < Group > ,
  115 | }
```


### 55 normalised tokens: `crates/cenc/src/table.rs` lines 110 to 127, and kopuz-a58c385 `daemon/core/src/queue_model.rs` lines 48 to 62

```text
  110 | ,
  111 | runs : usize ,
  112 | fragments : usize ,
  113 | }
  115 | impl SampleTable {
  117 | pub fn len ( & self ) -> usize {
  118 | self . offsets . len ( )
  119 | }
  122 | pub fn is_empty ( & self ) -> bool {
  123 | self . offsets . is_empty ( )
  124 | }
  127 | pub fn get ( & self
```


### 55 normalised tokens: `crates/cenc/src/tests/build.rs` lines 377 to 390, and kopuz-a58c385 `daemon/server/src/lyrics/request.rs` lines 24 to 35

```text
  377 | ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize ,
  386 | pub size : usize ,
  387 | pub duration : u32 ,
  388 | pub fragment : usize ,
  389 | pub entry : u32 ,
  390 | pub key_id :
```


### 54 normalised tokens: `crates/cenc/src/stream.rs` lines 308 to 318, and kopuz-a58c385 `daemon/server/src/ytmusic/browse.rs` lines 305 to 315

```text
  308 | {
  309 | Sample {
  310 | offset : sample . offset ,
  311 | size : sample . size ,
  312 | duration : sample . duration ,
  313 | fragment : sample . fragment ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs
```


### 52 normalised tokens: `crates/cenc/src/movie.rs` lines 33 to 50, and kopuz-a58c385 `shared/api/src/events.rs` lines 92 to 107

```text
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq
```


### 51 normalised tokens: `crates/cenc/src/tests/build.rs` lines 453 to 458, and kopuz-a58c385 `daemon/core/src/boot.rs` lines 313 to 320

```text
  453 | ) ;
  454 | trex . extend ( index . to_be_bytes ( ) ) ;
  455 | trex . extend ( duration . to_be_bytes ( ) ) ;
  456 | trex . extend ( size . to_be_bytes ( ) ) ;
  457 | trex . extend ( flags . to_be_bytes ( ) ) ;
  458 | mvex . extend ( full
```


### 47 normalised tokens: `crates/cenc/src/movie.rs` lines 36 to 50, and kopuz-a58c385 `shared/api/src/sources.rs` lines 146 to 155

```text
   36 | Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 46 normalised tokens: `crates/cenc/src/movie.rs` lines 17 to 32, and kopuz-a58c385 `shared/api/src/player.rs` lines 94 to 104

```text
   17 | at : usize ,
   18 | }
   21 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   22 | pub struct Track {
   24 | pub id : u32 ,
   26 | pub timescale : u32 ,
   28 | pub entries : Vec < SampleEntry > ,
   30 | pub defaults : TrackDefaults ,
   32 | pub
```


### 46 normalised tokens: `crates/cenc/src/movie.rs` lines 39 to 56, and kopuz-a58c385 `daemon/db/src/lib.rs` lines 232 to 244

```text
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   51 | pub struct SampleEntry {
   53 | pub at : usize ,
   56 | pub format :
```


### 46 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 20 to 27, and kopuz-a58c385 `daemon/db/src/backend/migrations.rs` lines 1400 to 1407

```text
   20 | ,
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples . to_vec ( ) ,
   27 | protected : sample . protected
```


### 45 normalised tokens: `crates/cenc/src/movie.rs` lines 32 to 46, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 173 to 182

```text
   32 | Group > ,
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
```


### 44 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 106, and kopuz-a58c385 `shared/api/src/library.rs` lines 182 to 190

```text
   94 | Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  104 | pub struct Pssh {
  106 | pub system_id :
```


### 44 normalised tokens: `crates/cenc/src/tests/build.rs` lines 375 to 385, and kopuz-a58c385 `shared/api/src/catalog.rs` lines 88 to 99

```text
  375 | ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize
```


### 43 normalised tokens: `crates/cenc/src/table.rs` lines 95 to 101, and kopuz-a58c385 `apps/kopuz/settings_appearance/src/lib.rs` lines 57 to 63

```text
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec <
```


### 42 normalised tokens: `crates/cenc/src/tests/build.rs` lines 219 to 233, and kopuz-a58c385 `apps/kopuz/store/src/jobs.rs` lines 18 to 29

```text
  219 | ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool ,
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec
```


### 42 normalised tokens: `crates/cenc/src/tests/build.rs` lines 304 to 310, and kopuz-a58c385 `apps/legacy/components/src/settings/remote_folders.rs` lines 13 to 18

```text
  304 | # [ derive ( Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
```


### 41 normalised tokens: `crates/cenc/src/lib.rs` lines 14 to 27, and kopuz-a58c385 `shared/proto/src/convert/mod.rs` lines 14 to 27

```text
   14 | mod boxes ;
   15 | mod claims ;
   16 | mod clear ;
   17 | mod error ;
   18 | mod fragment ;
   19 | mod indexer ;
   20 | mod movie ;
   21 | mod stream ;
   22 | mod table ;
   24 | # [ cfg ( test ) ]
   25 | mod tests ;
   27 | pub use clear ::
```


### 41 normalised tokens: `crates/cenc/src/tests/build.rs` lines 375 to 383, and kopuz-a58c385 `shared/api/src/library.rs` lines 51 to 59

```text
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 40 normalised tokens: `crates/cenc/src/indexer.rs` lines 43 to 49, and kopuz-a58c385 `apps/kopuz/palette/src/lib.rs` lines 195 to 201

```text
   43 | end ,
   44 | table : SampleTable :: default ( ) ,
   45 | mdats : Vec :: new ( ) ,
   46 | descriptions : HashMap :: new ( ) ,
   47 | claims : Claims :: default ( ) ,
   48 | pending_needs : None ,
   49 | steps :
```


### 40 normalised tokens: `crates/cenc/src/movie.rs` lines 5 to 17, and kopuz-a58c385 `daemon/server/src/ytmusic/accounts.rs` lines 7 to 18

```text
    5 | ;
    8 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
    9 | pub struct Movie {
   11 | pub tracks : Vec < Track > ,
   13 | pub pssh : Vec < Pssh > ,
   15 | pub end : usize ,
   17 | pub
```


### 40 normalised tokens: `crates/cenc/src/stream.rs` lines 326 to 330, and kopuz-a58c385 `daemon/discord-presence/src/lib.rs` lines 105 to 109

```text
  326 | fn lock ( & self ) -> MutexGuard < '_ , State > {
  327 | self . state . lock ( ) . unwrap_or_else ( | e | e . into_inner ( ) )
  328 | }
  330 | fn decrypt_all ( & self ,
```


### 40 normalised tokens: `crates/cenc/src/table.rs` lines 94 to 98, and kopuz-a58c385 `apps/kopuz/store/src/playing.rs` lines 6 to 10

```text
   94 | # [ derive ( Clone , Debug , Default , PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
```


### 39 normalised tokens: `crates/cenc/src/fragment.rs` lines 184 to 192, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 175 to 185

```text
  184 | ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool ,
  190 | }
  192 | fn tfhd ( data :
```


### 39 normalised tokens: `crates/cenc/src/fragment.rs` lines 505 to 512, and kopuz-a58c385 `apps/kopuz/store/src/jobs.rs` lines 185 to 192

```text
  505 | ,
  506 | scheme : protection . scheme ,
  507 | pattern : group . pattern ,
  508 | iv_size : group . iv_size ,
  509 | key_id : group . key_id ,
  510 | constant_iv : group . constant_iv ,
  511 | constant_iv_size : group . constant_iv_size ,
  512 | } )
```


### 39 normalised tokens: `crates/cenc/src/movie.rs` lines 32 to 41, and kopuz-a58c385 `shared/api/src/playlists.rs` lines 26 to 33

```text
   32 | groups : Vec < Group > ,
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
```


### 39 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 103, and kopuz-a58c385 `shared/api/src/playlists.rs` lines 30 to 36

```text
   94 | Copy , Debug , Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 39 normalised tokens: `crates/cenc/src/tests/build.rs` lines 216 to 223, and kopuz-a58c385 `daemon/server/src/ytmusic/discover.rs` lines 128 to 133

```text
  216 | Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool
```


### 38 normalised tokens: `crates/cenc/src/fragment.rs` lines 183 to 189, and kopuz-a58c385 `apps/kopuz/components/src/link.rs` lines 55 to 61

```text
  183 | struct TrackHeader {
  184 | track_id : u32 ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 50 to 68, and kopuz-a58c385 `daemon/core/src/integrations.rs` lines 286 to 294

```text
   50 | ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 611 to 614, and kopuz-a58c385 `shared/config/src/edit.rs` lines 298 to 301

```text
  611 | Some ( p ) ,
  612 | SeekFrom :: End ( d ) => self . len . checked_add_signed ( d ) ,
  613 | SeekFrom :: Current ( d ) => self . pos . checked_add_signed ( d ) ,
  614 | }
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 619 to 626, and kopuz-a58c385 `daemon/server/src/stream/range_source.rs` lines 325 to 332

```text
  619 | }
  621 | impl Drop for AudioStream {
  622 | fn drop ( & mut self ) {
  623 | self . shared . lock ( ) . abandoned = true ;
  624 | self . shared . changed . notify_all ( ) ;
  625 | }
  626 | }
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 223 to 234, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 364 to 374

```text
  223 | ,
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec < u8 > ,
  234 | pub subsamples : Vec <
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 254 to 262, and kopuz-a58c385 `shared/utils/src/color.rs` lines 7 to 13

```text
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool ,
  258 | }
  260 | impl RunSpec {
  262 | pub fn of ( count : u32 , size : u32 , iv_size : usize ) -> Self {
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 372 to 379, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/wayland/client.rs` lines 307 to 312

```text
  372 | Clone , Debug , Default ) ]
  373 | pub struct FileSpec {
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8
```


### 37 normalised tokens: `crates/cenc/src/fragment.rs` lines 184 to 190, and kopuz-a58c385 `apps/kopuz/components/src/selection.rs` lines 53 to 59

```text
  184 | track_id : u32 ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool ,
  190 | }
```


### 37 normalised tokens: `crates/cenc/src/movie.rs` lines 81 to 94, and kopuz-a58c385 `apps/legacy/components/src/layout/showcase.rs` lines 7 to 15

```text
   81 | Copy , Debug , Hash , PartialEq , Eq ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
   91 | }
   94 | # [ derive ( Clone , Copy , Debug , Default , Hash
```


### 37 normalised tokens: `crates/cenc/src/stream.rs` lines 46 to 61, and kopuz-a58c385 `apps/kopuz/metadata_dialog/src/lib.rs` lines 67 to 75

```text
   46 | ,
   47 | samples : Samples ,
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool
```


### 37 normalised tokens: `crates/cenc/src/table.rs` lines 94 to 99, and kopuz-a58c385 `apps/legacy/hooks/src/ui_prefs.rs` lines 19 to 25

```text
   94 | PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 182 to 190, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 365 to 374

```text
  182 | }
  184 | # [ derive ( Clone , Debug ) ]
  185 | pub struct TrackSpec {
  186 | pub id : u32 ,
  187 | pub timescale : u32 ,
  188 | pub entries : Vec < Entry > ,
  190 | pub trex : Option <
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 213 to 221, and kopuz-a58c385 `apps/legacy/components/src/common/controls/dots_menu.rs` lines 60 to 67

```text
  213 | }
  214 | }
  216 | # [ derive ( Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 218 to 227, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 761 to 768

```text
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool ,
  224 | }
  227 | # [ derive ( Clone , Debug
```


### 36 normalised tokens: `crates/cenc/src/lib.rs` lines 27 to 30, and kopuz-a58c385 `apps/kopuz/components/src/lib.rs` lines 147 to 151

```text
   27 | last_sample_duration , make_clear , write_key_ids } ;
   28 | pub use error :: Error ;
   29 | pub use indexer :: Indexer ;
   30 | pub use movie :: { Movie , Pattern , Protection , Pssh , SampleEntry , Scheme ,
```


### 36 normalised tokens: `crates/cenc/src/stream.rs` lines 50 to 63, and kopuz-a58c385 `daemon/core/src/spotify.rs` lines 45 to 56

```text
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 113 to 121, and kopuz-a58c385 `daemon/server/src/applemusic/cenc.rs` lines 324 to 336

```text
  113 | }
  116 | # [ derive ( Clone , Debug ) ]
  117 | pub struct Seig {
  118 | pub protected : bool ,
  119 | pub iv_size : u8 ,
  120 | pub key_id : [ u8 ; 16 ] ,
  121 | pub pattern :
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 249 to 257, and kopuz-a58c385 `daemon/core/src/external.rs` lines 45 to 53

```text
  249 | Clone , Debug ) ]
  250 | pub struct RunSpec {
  251 | pub samples : Vec < SampleSpec > ,
  253 | pub data_offset : bool ,
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 291 to 304, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 381 to 392

```text
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
  302 | }
  304 | # [ derive ( Clone , Debug
```


### 36 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 18 to 20, and kopuz-a58c385 `daemon/kopuzd/tests/contract.rs` lines 2056 to 2058

```text
   18 | let mut starts : Vec < usize > = table . iter ( ) . map ( | s | s . fragment ) . collect ( ) ;
   19 | starts . dedup ( ) ;
   20 | starts
```


### 35 normalised tokens: `crates/cenc/src/boxes.rs` lines 335 to 335, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/x11/clipboard.rs` lines 367 to 368

```text
  335 | 4 ) . map ( | b | u32 :: from_be_bytes ( [ b [ 0 ] , b [ 1 ] , b [ 2 ] , b [ 3 ] ] ) )
```


### 35 normalised tokens: `crates/cenc/src/clear.rs` lines 117 to 121, and kopuz-a58c385 `apps/kopuz/store/src/downloads.rs` lines 200 to 205

```text
  117 | {
  118 | let Some ( entry ) = track . entries . iter ( ) . find ( | entry | entry . at == entry_box . at ) else {
  119 | continue ;
  120 | } ;
  121 | if
```


### 35 normalised tokens: `crates/cenc/src/lib.rs` lines 30 to 32, and kopuz-a58c385 `apps/kopuz/theme/src/lib.rs` lines 18 to 20

```text
   30 | Pattern , Protection , Pssh , SampleEntry , Scheme , Track , TrackDefaults } ;
   31 | pub use stream :: { AudioStream , Decrypt } ;
   32 | pub use table :: { Sample , SampleTable } ;
```


### 35 normalised tokens: `crates/cenc/src/stream.rs` lines 53 to 68, and kopuz-a58c385 `apps/kopuz/components/src/seek_bar.rs` lines 99 to 106

```text
   53 | ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize ,
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 44 to 51, and kopuz-a58c385 `daemon/server/src/nextcloud/tree.rs` lines 43 to 49

```text
   44 | pub ( crate ) struct Description {
   45 | pub entry : u32 ,
   46 | pub protected : bool ,
   47 | pub scheme : Scheme ,
   48 | pub pattern : Pattern ,
   50 | pub iv_size : u8 ,
   51 | pub key_id :
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 174 to 181, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 759 to 766

```text
  174 | }
  177 | pub ( crate ) fn fragments ( & self ) -> & [ Fragment ] {
  178 | & self . fragments
  179 | }
  181 | pub ( crate ) fn mark ( & self ) -> Mark {
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 298 to 302, and kopuz-a58c385 `apps/kopuz/components/src/selection.rs` lines 300 to 304

```text
  298 | fn shrink_to_fit ( & mut self ) {
  299 | self . offsets . shrink_to_fit ( ) ;
  300 | self . fields . shrink_to_fit ( ) ;
  301 | self . description_of . shrink_to_fit ( ) ;
  302 | self . run_counts
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 290 to 304, and kopuz-a58c385 `apps/kopuz/components/src/track_meta.rs` lines 211 to 228

```text
  290 | }
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
  302 | }
  304 | # [ derive ( Clone
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 302 to 309, and kopuz-a58c385 `daemon/server/src/applemusic/cenc.rs` lines 343 to 354

```text
  302 | }
  304 | # [ derive ( Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 310 to 317, and kopuz-a58c385 `daemon/server/src/download_queue.rs` lines 32 to 37

```text
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux ,
  317 | pub sbgp : Vec
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 400 to 409, and kopuz-a58c385 `daemon/server/src/ytmusic/browse/search.rs` lines 112 to 120

```text
  400 | Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
  409 | pub
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 175 to 177, and kopuz-a58c385 `apps/kopuz/now_playing/src/lyrics_view.rs` lines 735 to 738

```text
  175 | ) ;
  176 | let entries : Vec < u32 > = table . iter ( ) . map ( | s | s . entry ) . collect ( ) ;
  177 | assert_eq ! ( entries ,
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 198 to 200, and kopuz-a58c385 `apps/kopuz/now_playing/src/lyrics_view.rs` lines 735 to 738

```text
  198 | ) ;
  199 | let durations : Vec < u32 > = table . iter ( ) . map ( | s | s . duration ) . collect ( ) ;
  200 | assert_eq ! ( durations ,
```


### 35 normalised tokens: `crates/cenc/src/tests/other.rs` lines 94 to 100, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 1195 to 1201

```text
   94 | ) ;
   95 | assert_eq ! ( source . to_string ( ) , error . to_string ( ) ) ;
   96 | }
   98 | # [ test ]
   99 | fn an_empty_file_and_a_file_of_only_an_ftyp ( ) {
  100 | assert_eq ! ( Movie :: read (
```


### 34 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 11, and kopuz-a58c385 `apps/kopuz/components/src/folders.rs` lines 10 to 15

```text
    8 | Kind , Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track
```


### 34 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 11, and kopuz-a58c385 `apps/kopuz/components/src/states.rs` lines 10 to 15

```text
    8 | Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track } ;
```


### 34 normalised tokens: `crates/cenc/src/error.rs` lines 22 to 28, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 52 to 58

```text
   22 | ) ,
   23 | }
   24 | }
   25 | }
   27 | impl fmt :: Display for Error {
   28 | fn fmt ( & self , f : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
```


### 34 normalised tokens: `crates/cenc/src/fragment.rs` lines 255 to 261, and kopuz-a58c385 `apps/cli/src/lib.rs` lines 456 to 461

```text
  255 | let track = cx
  256 | . tracks
  257 | . iter ( )
  258 | . find ( | track | track . id == head . track_id )
  259 | . ok_or_else ( || {
  260 | Error :: new (
  261 | tfhd_box
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 200 to 214, and kopuz-a58c385 `apps/kopuz/primitives/src/focus.rs` lines 54 to 64

```text
  200 | }
  203 | pub fn len ( & self ) -> u64 {
  204 | self . len
  205 | }
  208 | pub fn is_empty ( & self ) -> bool {
  209 | self . len == 0
  210 | }
  214 | pub fn with_samples
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 313 to 319, and kopuz-a58c385 `apps/legacy/components/src/layout/normal/showcase.rs` lines 249 to 255

```text
  313 | ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs ,
  319 | protected : sample . protected
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 608 to 611, and kopuz-a58c385 `daemon/server/src/audio_cd/mod.rs` lines 156 to 159

```text
  608 | {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
  610 | let pos = match to {
  611 | SeekFrom :: Start ( p ) => Some
```


### 34 normalised tokens: `crates/cenc/src/table.rs` lines 34 to 44, and kopuz-a58c385 `shared/api/src/player.rs` lines 57 to 63

```text
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
   44 | pub
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 339 to 347, and kopuz-a58c385 `apps/legacy/components/src/common/controls/dots_menu.rs` lines 63 to 68

```text
  339 | Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1029 to 1034, and kopuz-a58c385 `daemon/db/src/backend/rows.rs` lines 104 to 109

```text
 1029 | Resolved {
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv . clone (
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1030 to 1034, and kopuz-a58c385 `daemon/db/src/backend/migrations.rs` lines 1402 to 1406

```text
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv . clone ( ) ,
```


### 33 normalised tokens: `crates/cenc/src/clear.rs` lines 7 to 11, and kopuz-a58c385 `apps/kopuz/discover/src/landing.rs` lines 20 to 25

```text
    7 | boxes :: {
    8 | Header , Kind , Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate ::
```


### 33 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 13, and kopuz-a58c385 `apps/kopuz/components/src/pick_list.rs` lines 12 to 18

```text
    8 | lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track } ;
   13 | const FREE : Kind =
```


### 33 normalised tokens: `crates/cenc/src/fragment.rs` lines 108 to 117, and kopuz-a58c385 `daemon/core/src/session/mod.rs` lines 2564 to 2571

```text
  108 | ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option < Header > ,
  114 | groups : Vec < Group > ,
  115 | }
  117 | fn locate (
```


### 33 normalised tokens: `crates/cenc/src/indexer.rs` lines 18 to 27, and kopuz-a58c385 `daemon/server/src/ytmusic/tracking.rs` lines 44 to 50

```text
   18 | # [ derive ( Debug ) ]
   19 | pub struct Indexer {
   20 | tracks : Vec < Track > ,
   22 | position : usize ,
   24 | scanned_to : usize ,
   25 | table : SampleTable ,
   27 | mdats : Vec
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 46 to 57, and kopuz-a58c385 `apps/kopuz/components/src/settings.rs` lines 566 to 571

```text
   46 | data : Vec < u8 > ,
   47 | samples : Samples ,
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 271 to 274, and kopuz-a58c385 `daemon/server/src/ytmusic/mix.rs` lines 52 to 55

```text
  271 | fn table ( & self ) -> Option < & SampleTable > {
  272 | match self {
  273 | Self :: Whole ( table ) => Some ( table ) ,
  274 | Self :: Arriving ( Some
```


### 33 normalised tokens: `crates/cenc/src/table.rs` lines 99 to 103, and kopuz-a58c385 `apps/kopuz/components/src/track_header.rs` lines 78 to 82

```text
   99 | ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec < ( u32 , u32 ) > ,
  102 | descriptions : Vec < Description > ,
  103 | fragments : Vec < Fragment > ,
```


### 33 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 187, and kopuz-a58c385 `daemon/core/src/session/state.rs` lines 157 to 161

```text
  183 | ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments : self . fragments . len ( ) ,
  187 | }
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 340 to 349, and kopuz-a58c385 `daemon/core/src/session/mod.rs` lines 46 to 52

```text
  340 | struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 370 to 377, and kopuz-a58c385 `daemon/core/src/external.rs` lines 42 to 49

```text
  370 | }
  372 | # [ derive ( Clone , Debug , Default ) ]
  373 | pub struct FileSpec {
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1267 to 1270, and kopuz-a58c385 `apps/legacy/hooks/src/use_db_queries.rs` lines 68 to 71

```text
 1267 | corpus ( )
 1268 | . into_iter ( )
 1269 | . find ( | ( found , _ ) | * found == name )
 1270 | . map ( | ( _ , spec ) | spec
```


### 33 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 33 to 36, and kopuz-a58c385 `daemon/core/src/persistence.rs` lines 221 to 224

```text
   33 | clear_movie
   34 | . tracks
   35 | . iter ( )
   36 | . all ( | t | t . entries . iter ( ) . all ( | e | e . protection . is_none ( )
```


### 33 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 18 to 22, and kopuz-a58c385 `daemon/core/src/library/mod.rs` lines 1204 to 1208

```text
   18 | table . iter ( ) . map ( | s | s . fragment ) . collect ( ) ;
   19 | starts . dedup ( ) ;
   20 | starts
   21 | } ;
   22 | assert_eq ! ( fragments
```


### 33 normalised tokens: `crates/cenc/src/tests/other.rs` lines 94 to 99, and kopuz-a58c385 `apps/kopuz/store/src/notices.rs` lines 257 to 262

```text
   94 | error . clone ( ) ) ;
   95 | assert_eq ! ( source . to_string ( ) , error . to_string ( ) ) ;
   96 | }
   98 | # [ test ]
   99 | fn an_empty_file_and_a_file_of_only_an_ftyp ( ) {
```


### 32 normalised tokens: `crates/cenc/examples/index_cost.rs` lines 15 to 18, and kopuz-a58c385 `daemon/core/src/offline.rs` lines 3 to 6

```text
   15 | ;
   16 | use std :: hint :: black_box ;
   17 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
   18 | use std :: time :: { Duration , Instant } ;
```


### 32 normalised tokens: `crates/cenc/src/error.rs` lines 24 to 29, and kopuz-a58c385 `apps/kopuz/store/src/target.rs` lines 66 to 71

```text
   24 | }
   25 | }
   27 | impl fmt :: Display for Error {
   28 | fn fmt ( & self , f : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
   29 | write
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 100 to 108, and kopuz-a58c385 `daemon/player/src/engine/worker.rs` lines 111 to 119

```text
  100 | }
  104 | # [ derive ( Default ) ]
  105 | struct TrafBoxes {
  106 | tfhd : Option < Header > ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 390 to 393, and kopuz-a58c385 `apps/kopuz/components/src/karaoke.rs` lines 118 to 119

```text
  390 | let default_duration = head
  391 | . default_duration
  392 | . unwrap_or ( track . defaults . sample_duration ) ;
  393 | let default_size = head . default_size . unwrap_or ( track . defaults . sample_size ) ;
```


### 32 normalised tokens: `crates/cenc/src/lib.rs` lines 30 to 40, and kopuz-a58c385 `apps/kopuz/store/src/lib.rs` lines 48 to 51

```text
   30 | Pssh , SampleEntry , Scheme , Track , TrackDefaults } ;
   31 | pub use stream :: { AudioStream , Decrypt } ;
   32 | pub use table :: { Sample , SampleTable } ;
   40 | pub
```


### 32 normalised tokens: `crates/cenc/src/movie.rs` lines 100 to 108, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/mediadrm.rs` lines 30 to 41

```text
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  104 | pub struct Pssh {
  106 | pub system_id : [ u8 ; 16 ] ,
  108 | pub key_ids : Vec <
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 32 to 43, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 408 to 415

```text
   32 | ,
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 88 to 97, and kopuz-a58c385 `apps/kopuz/components/src/selection.rs` lines 185 to 192

```text
   88 | }
   94 | # [ derive ( Clone , Debug , Default , PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 315 to 323, and kopuz-a58c385 `apps/kopuz/library/src/filters.rs` lines 17 to 24

```text
  315 | ,
  317 | pub sbgp : Vec < ( u32 , u32 ) > ,
  319 | pub sgpd : Vec < Seig > ,
  320 | }
  322 | impl TrafSpec {
  323 | pub fn new ( track_id : u32
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 880 to 880, and kopuz-a58c385 `daemon/server/src/ytmusic/fixture_tests.rs` lines 191 to 191

```text
  880 | let samples : Vec < & SampleSpec > = traf . runs . iter ( ) . flat_map ( | run | & run . samples ) . collect ( ) ;
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 935 to 941, and kopuz-a58c385 `apps/legacy/kopuz/src/updates.rs` lines 216 to 223

```text
  935 | fn expected (
  936 | spec : & FragmentSpec ,
  937 | movie : & MovieSpec ,
  938 | moof_at : usize ,
  939 | layouts : & [ TrafLayout ] ,
  940 | ) -> Vec < Expected > {
  941 | let
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1314 to 1314, and kopuz-a58c385 `daemon/server/src/applemusic/cenc.rs` lines 4 to 4

```text
 1314 | u32 :: from_be_bytes ( [ bytes [ at ] , bytes [ at + 1 ] , bytes [ at + 2 ] , bytes [ at + 3 ] ] )
```


### 32 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 17 to 18, and kopuz-a58c385 `apps/kopuz/page_albums/src/detail.rs` lines 553 to 556

```text
   17 | let fragments : Vec < usize > = {
   18 | let mut starts : Vec < usize > = table . iter ( ) . map ( | s | s . fragment
```


### 32 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 177 to 179, and kopuz-a58c385 `daemon/db/tests/persistence.rs` lines 148 to 151

```text
  177 | ] ) ;
  178 | assert_eq ! ( movie . pssh . len ( ) , 1 ) ;
  179 | assert_eq ! ( movie . pssh [ 0 ] . key_ids , vec ! [
```


### 32 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 23 to 28, and kopuz-a58c385 `shared/proto/src/convert/config.rs` lines 475 to 480

```text
   23 | ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples . to_vec ( ) ,
   27 | protected : sample . protected ,
   28 | scheme : scheme_name
```


### 31 normalised tokens: `crates/cenc/src/boxes.rs` lines 188 to 199, and kopuz-a58c385 `apps/kopuz/store/src/setup.rs` lines 67 to 76

```text
  188 | body ,
  189 | }
  190 | }
  193 | pub fn at ( & self ) -> usize {
  194 | self . at
  195 | }
  198 | pub fn left ( & self ) -> usize {
  199 | self . header
```


### 31 normalised tokens: `crates/cenc/src/fragment.rs` lines 8 to 13, and kopuz-a58c385 `apps/kopuz/page_home/src/shelf.rs` lines 5 to 10

```text
    8 | ;
    9 | use crate :: movie :: { Group , SampleEntry , Track , sgpd } ;
   10 | use crate :: table :: { Description , SampleTable } ;
   13 | pub ( crate )
```


### 31 normalised tokens: `crates/cenc/src/indexer.rs` lines 7 to 10, and kopuz-a58c385 `apps/legacy/components/src/track/actions.rs` lines 5 to 8

```text
    7 | , Fault } ;
    8 | use crate :: movie :: { Movie , Track } ;
    9 | use crate :: table :: { Description , Fragment , SampleTable } ;
   10 | use crate ::
```


### 31 normalised tokens: `crates/cenc/src/indexer.rs` lines 10 to 25, and kopuz-a58c385 `apps/kopuz/topbar/src/switcher.rs` lines 34 to 42

```text
   10 | ;
   18 | # [ derive ( Debug ) ]
   19 | pub struct Indexer {
   20 | tracks : Vec < Track > ,
   22 | position : usize ,
   24 | scanned_to : usize ,
   25 | table : SampleTable ,
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 77 to 90, and kopuz-a58c385 `apps/kopuz/store/src/target.rs` lines 118 to 127

```text
   77 | ,
   78 | }
   81 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 84 to 95, and kopuz-a58c385 `shared/config/src/lib.rs` lines 121 to 129

```text
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
   91 | }
   94 | # [ derive ( Clone , Copy , Debug , Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 530 to 535, and kopuz-a58c385 `daemon/server/src/smb/stream.rs` lines 40 to 45

```text
  530 | }
  531 | }
  533 | impl Read for AudioStream {
  534 | fn read ( & mut self , buf : & mut [ u8 ] ) -> io :: Result < usize > {
  535 | if
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 743 to 746, and kopuz-a58c385 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  743 | ) ;
  744 | let mut out = Vec :: new ( ) ;
  745 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  746 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 893 to 896, and kopuz-a58c385 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  893 | ) ;
  894 | let mut out = Vec :: new ( ) ;
  895 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  896 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 911 to 914, and kopuz-a58c385 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  911 | ) ;
  912 | let mut out = Vec :: new ( ) ;
  913 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  914 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 74 to 80, and kopuz-a58c385 `apps/kopuz/store/src/downloads.rs` lines 42 to 48

```text
   74 | struct Fragment {
   75 | pub start : usize ,
   76 | pub first_sample : usize ,
   77 | }
   80 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 80 to 88, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/wayland/client.rs` lines 117 to 123

```text
   80 | Copy , Debug , Default , PartialEq , Eq ) ]
   81 | struct Fields {
   82 | size : u32 ,
   83 | duration : u32 ,
   85 | iv_at : u32 ,
   87 | runs_at : u32 ,
   88 | }
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 81 to 94, and kopuz-a58c385 `apps/legacy/kopuz/src/updates.rs` lines 21 to 28

```text
   81 | struct Fields {
   82 | size : u32 ,
   83 | duration : u32 ,
   85 | iv_at : u32 ,
   87 | runs_at : u32 ,
   88 | }
   94 | # [ derive ( Clone , Debug , Default , PartialEq
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 288 to 301, and kopuz-a58c385 `apps/kopuz/library/src/filters.rs` lines 162 to 173

```text
  288 | }
  289 | flags
  290 | }
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 310 to 317, and kopuz-a58c385 `daemon/db/src/lib.rs` lines 56 to 62

```text
  310 | u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux ,
  317 | pub sbgp : Vec <
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 341 to 349, and kopuz-a58c385 `apps/kopuz/page_home/src/data.rs` lines 35 to 40

```text
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 396 to 403, and kopuz-a58c385 `apps/legacy/hooks/src/use_db_queries.rs` lines 31 to 39

```text
  396 | ,
  397 | }
  400 | # [ derive ( Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 402 to 409, and kopuz-a58c385 `apps/legacy/components/src/playback/controls.rs` lines 121 to 126

```text
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
  409 | pub fn build ( spec :
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 416 to 420, and kopuz-a58c385 `apps/legacy/components/src/navigation/controller.rs` lines 110 to 114

```text
  416 | cleared ) ;
  417 | bytes . extend ( built . bytes ) ;
  418 | samples . extend ( built . samples ) ;
  419 | fragments . push ( built . moof_at ) ;
  420 | }
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1268 to 1270, and kopuz-a58c385 `shared/config/src/store.rs` lines 269 to 271

```text
 1268 | . into_iter ( )
 1269 | . find ( | ( found , _ ) | * found == name )
 1270 | . map ( | ( _ , spec ) | spec )
```


### 31 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 39 to 40, and kopuz-a58c385 `daemon/core/src/prefs.rs` lines 102 to 106

```text
   39 | ) > = table . iter ( ) . map ( | s | ( s . offset , s . size ) ) . collect ( ) ;
   40 | let clear_places
```


### 31 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 205 to 207, and kopuz-a58c385 `apps/legacy/pages/src/server/browse.rs` lines 620 to 622

```text
  205 | ) ;
  206 | assert_eq ! ( movie . pssh . len ( ) , 2 ) ;
  207 | assert ! ( movie . pssh [ 0 ] . key_ids . is_empty ( )
```


### 31 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 206 to 208, and kopuz-a58c385 `daemon/core/src/sources/removable.rs` lines 298 to 300

```text
  206 | ) ;
  207 | assert ! ( movie . pssh [ 0 ] . key_ids . is_empty ( ) ) ;
  208 | assert_eq ! ( movie . pssh [ 0 ] . data ,
```


### 31 normalised tokens: `crates/cenc/src/tests/other.rs` lines 154 to 156, and kopuz-a58c385 `daemon/core/src/queue_model.rs` lines 805 to 808

```text
  154 | ) ;
  155 | assert_eq ! ( table . len ( ) , 3 ) ;
  156 | assert ! ( table . iter ( ) . all ( | s | s . offset
```


### 31 normalised tokens: `crates/cenc/tests/allocation.rs` lines 5 to 7, and kopuz-a58c385 `apps/kopuz/page_settings/src/lib.rs` lines 5 to 7

```text
    5 | GlobalAlloc , Layout , System } ;
    6 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    7 | use std :: sync :: { Mutex , MutexGuard } ;
```


### 31 normalised tokens: `crates/cenc/tests/allocation.rs` lines 121 to 123, and kopuz-a58c385 `apps/kopuz/components/src/schema_form.rs` lines 340 to 342

```text
  121 | read ( & file ) . unwrap ( ) . unwrap ( ) ;
  122 | let mut indexer = cenc :: Indexer :: new ( & movie ) ;
  123 | for end in
```


### 30 normalised tokens: `crates/cenc/examples/index_cost.rs` lines 15 to 18, and kopuz-a58c385 `daemon/player/src/engine/worker.rs` lines 10 to 13

```text
   15 | Layout , System } ;
   16 | use std :: hint :: black_box ;
   17 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
   18 | use std :: time ::
```


### 30 normalised tokens: `crates/cenc/src/boxes.rs` lines 190 to 199, and kopuz-a58c385 `apps/kopuz/store/src/query.rs` lines 155 to 164

```text
  190 | }
  193 | pub fn at ( & self ) -> usize {
  194 | self . at
  195 | }
  198 | pub fn left ( & self ) -> usize {
  199 | self . header . end
```


### 30 normalised tokens: `crates/cenc/src/boxes.rs` lines 286 to 292, and kopuz-a58c385 `shared/api/src/artwork.rs` lines 60 to 66

```text
  286 | end : usize ,
  287 | }
  289 | impl Lenient {
  290 | pub fn new ( at : usize , end : usize ) -> Self {
  291 | Self { at , end }
  292 | }
```


### 30 normalised tokens: `crates/cenc/src/claims.rs` lines 97 to 103, and kopuz-a58c385 `daemon/server/src/audio_cd/native.rs` lines 699 to 707

```text
   97 | ) ) ;
   98 | }
  100 | # [ test ]
  101 | fn a_fragment_given_back_owns_nothing_and_one_kept_owns_its_ranges ( ) {
  102 | let mut claims = Claims :: default ( ) ;
  103 | assert ! ( claims . claim (
```


### 30 normalised tokens: `crates/cenc/src/error.rs` lines 1 to 12, and kopuz-a58c385 `daemon/radio/src/provider.rs` lines 7 to 12

```text
    1 | use std :: fmt ;
    7 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
    8 | pub struct Error {
   10 | pub offset : usize ,
   12 | pub kind :
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 151 to 154, and kopuz-a58c385 `daemon/core/src/prefs.rs` lines 18 to 20

```text
  151 | Header , parent : Header ) -> Result < ( ) , Error > {
  152 | if slot . is_some ( ) {
  153 | return Err ( Error :: new (
  154 | parent
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 476 to 481, and kopuz-a58c385 `apps/legacy/components/src/layout/vaxry/showcase.rs` lines 245 to 250

```text
  476 | protected : true ,
  477 | scheme : protection . scheme ,
  478 | pattern : protection . pattern ,
  479 | iv_size : protection . iv_size ,
  480 | key_id : protection . key_id ,
  481 | constant_iv ,
```


### 30 normalised tokens: `crates/cenc/src/lib.rs` lines 28 to 30, and kopuz-a58c385 `daemon/player/src/engine/mod.rs` lines 16 to 19

```text
   28 | use error :: Error ;
   29 | pub use indexer :: Indexer ;
   30 | pub use movie :: { Movie , Pattern , Protection , Pssh , SampleEntry , Scheme , Track ,
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 62 to 71, and kopuz-a58c385 `daemon/db/src/lib.rs` lines 221 to 230

```text
   62 | children_at : usize ,
   63 | }
   66 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   67 | pub struct Protection {
   69 | pub scheme : Scheme ,
   71 | pub key_id :
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 178 to 181, and kopuz-a58c385 `daemon/server/src/source/apple_music.rs` lines 270 to 274

```text
  178 | ) -> Result < Movie , Error > {
  179 | let mut tracks = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 179 to 181, and kopuz-a58c385 `daemon/server/src/nextcloud/mod.rs` lines 186 to 188

```text
  179 | let mut tracks = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let mut defaults = Vec :: new ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 179 to 182, and kopuz-a58c385 `daemon/server/src/ytmusic/discover.rs` lines 530 to 533

```text
  179 | = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let mut defaults = Vec :: new ( ) ;
  182 | let mut children
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 21 to 25, and kopuz-a58c385 `daemon/core/src/sources.rs` lines 9 to 13

```text
   21 | Read , Seek , SeekFrom } ;
   22 | use std :: sync :: { Arc , Condvar , Mutex , MutexGuard } ;
   23 | use std :: time :: Duration ;
   25 | use
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 94 to 101, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 141 to 147

```text
   94 | pub struct Feed {
   95 | shared : Arc < Shared > ,
   96 | }
   98 | impl AudioStream {
  100 | pub fn plain ( data : Vec < u8 > ) -> Self {
  101 | Self
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 604 to 609, and kopuz-a58c385 `daemon/server/src/smb/stream.rs` lines 90 to 95

```text
  604 | Ok ( n )
  605 | }
  606 | }
  608 | impl Seek for AudioStream {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
```


### 30 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 186, and kopuz-a58c385 `daemon/db/src/backend/cfg_store.rs` lines 492 to 495

```text
  183 | . len ( ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments : self . fragments
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 180 to 188, and kopuz-a58c385 `apps/kopuz/store/src/notices.rs` lines 88 to 97

```text
  180 | )
  181 | }
  182 | }
  184 | # [ derive ( Clone , Debug ) ]
  185 | pub struct TrackSpec {
  186 | pub id : u32 ,
  187 | pub timescale : u32 ,
  188 | pub entries : Vec
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 192 to 202, and kopuz-a58c385 `apps/legacy/hooks/src/downloads.rs` lines 24 to 31

```text
  192 | groups : Vec < Seig > ,
  194 | pub sgpd_version : u8 ,
  196 | pub version1_headers : bool ,
  198 | pub visual : bool ,
  199 | }
  201 | impl TrackSpec {
  202 | pub fn audio (
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 194 to 202, and kopuz-a58c385 `shared/utils/src/color.rs` lines 7 to 13

```text
  194 | pub sgpd_version : u8 ,
  196 | pub version1_headers : bool ,
  198 | pub visual : bool ,
  199 | }
  201 | impl TrackSpec {
  202 | pub fn audio ( id : u32 , entries : Vec
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 229 to 234, and kopuz-a58c385 `apps/kopuz/library/src/filters.rs` lines 15 to 18

```text
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec < u8 > ,
  234 | pub subsamples : Vec < ( u16 , u32 ) > ,
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 333 to 340, and kopuz-a58c385 `daemon/core/src/offline_cache.rs` lines 136 to 143

```text
  333 | Vec :: new ( ) ,
  334 | sgpd : Vec :: new ( ) ,
  335 | }
  336 | }
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 337 to 343, and kopuz-a58c385 `apps/legacy/hooks/src/use_db_queries.rs` lines 32 to 39

```text
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 343 to 353, and kopuz-a58c385 `apps/legacy/hooks/src/downloads.rs` lines 24 to 31

```text
  343 | gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool ,
  350 | }
  352 | impl FragmentSpec {
  353 | pub fn new (
```


### 30 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 39 to 41, and kopuz-a58c385 `apps/legacy/pages/src/settings/sections.rs` lines 126 to 130

```text
   39 | ) ) . collect ( ) ;
   40 | let clear_places : Vec < ( usize , usize ) > =
   41 | clear_table . iter ( ) . map ( | s |
```


### 30 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 49 to 52, and kopuz-a58c385 `daemon/radio/src/browser.rs` lines 358 to 361

```text
   49 | ) ;
   50 | assert_eq ! ( movie . tracks . len ( ) , 1 ) ;
   51 | let track = & movie . tracks [ 0 ] ;
   52 | assert_eq ! (
```


### 30 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 169 to 174, and kopuz-a58c385 `daemon/server/src/subsonic.rs` lines 984 to 994

```text
  169 | table . iter ( ) . all ( | s | s . key_id == OTHER_KEY ) ) ;
  170 | }
  172 | # [ test ]
  173 | fn several_sample_entries_for_spatial_audio ( ) {
  174 | let
```


### 67 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 18 to 28, and kopuz-1d8b323 `shared/proto/src/convert/sources.rs` lines 17 to 27

```text
   18 | offset : sample . offset ,
   19 | size : sample . size ,
   20 | duration : sample . duration ,
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples . to_vec ( ) ,
   27 | protected : sample . protected ,
   28 | scheme : scheme_name
```


### 64 normalised tokens: `crates/cenc/src/tests/build.rs` lines 376 to 389, and kopuz-1d8b323 `daemon/server/src/lyrics/request.rs` lines 3 to 18

```text
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize ,
  386 | pub size : usize ,
  387 | pub duration : u32 ,
  388 | pub fragment : usize ,
  389 | pub entry : u32 ,
```


### 60 normalised tokens: `crates/cenc/src/tests/build.rs` lines 304 to 315, and kopuz-1d8b323 `shared/api/src/catalog.rs` lines 81 to 94

```text
  304 | Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux
```


### 59 normalised tokens: `crates/cenc/src/table.rs` lines 95 to 103, and kopuz-1d8b323 `daemon/player/src/engine/rt.rs` lines 122 to 131

```text
   95 | struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec < ( u32 , u32 ) > ,
  102 | descriptions : Vec < Description > ,
  103 | fragments : Vec
```


### 59 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 16 to 26, and kopuz-1d8b323 `daemon/server/src/ytmusic/browse.rs` lines 305 to 315

```text
   16 | ) -> Expected {
   17 | Expected {
   18 | offset : sample . offset ,
   19 | size : sample . size ,
   20 | duration : sample . duration ,
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples
```


### 56 normalised tokens: `crates/cenc/src/fragment.rs` lines 104 to 115, and kopuz-1d8b323 `daemon/server/src/ytmusic/browse/items.rs` lines 121 to 132

```text
  104 | Default ) ]
  105 | struct TrafBoxes {
  106 | tfhd : Option < Header > ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option < Header > ,
  114 | groups : Vec < Group > ,
  115 | }
```


### 55 normalised tokens: `crates/cenc/src/table.rs` lines 110 to 127, and kopuz-1d8b323 `daemon/core/src/queue_model.rs` lines 48 to 62

```text
  110 | ,
  111 | runs : usize ,
  112 | fragments : usize ,
  113 | }
  115 | impl SampleTable {
  117 | pub fn len ( & self ) -> usize {
  118 | self . offsets . len ( )
  119 | }
  122 | pub fn is_empty ( & self ) -> bool {
  123 | self . offsets . is_empty ( )
  124 | }
  127 | pub fn get ( & self
```


### 55 normalised tokens: `crates/cenc/src/tests/build.rs` lines 377 to 390, and kopuz-1d8b323 `daemon/server/src/lyrics/request.rs` lines 24 to 35

```text
  377 | ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize ,
  386 | pub size : usize ,
  387 | pub duration : u32 ,
  388 | pub fragment : usize ,
  389 | pub entry : u32 ,
  390 | pub key_id :
```


### 54 normalised tokens: `crates/cenc/src/stream.rs` lines 308 to 318, and kopuz-1d8b323 `daemon/server/src/ytmusic/browse.rs` lines 305 to 315

```text
  308 | {
  309 | Sample {
  310 | offset : sample . offset ,
  311 | size : sample . size ,
  312 | duration : sample . duration ,
  313 | fragment : sample . fragment ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs
```


### 52 normalised tokens: `crates/cenc/src/movie.rs` lines 33 to 50, and kopuz-1d8b323 `shared/api/src/events.rs` lines 92 to 107

```text
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq
```


### 51 normalised tokens: `crates/cenc/src/tests/build.rs` lines 453 to 458, and kopuz-1d8b323 `daemon/core/src/boot.rs` lines 314 to 321

```text
  453 | ) ;
  454 | trex . extend ( index . to_be_bytes ( ) ) ;
  455 | trex . extend ( duration . to_be_bytes ( ) ) ;
  456 | trex . extend ( size . to_be_bytes ( ) ) ;
  457 | trex . extend ( flags . to_be_bytes ( ) ) ;
  458 | mvex . extend ( full
```


### 47 normalised tokens: `crates/cenc/src/movie.rs` lines 36 to 50, and kopuz-1d8b323 `shared/api/src/sources.rs` lines 148 to 157

```text
   36 | Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 46 normalised tokens: `crates/cenc/src/movie.rs` lines 17 to 32, and kopuz-1d8b323 `shared/api/src/player.rs` lines 94 to 104

```text
   17 | at : usize ,
   18 | }
   21 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   22 | pub struct Track {
   24 | pub id : u32 ,
   26 | pub timescale : u32 ,
   28 | pub entries : Vec < SampleEntry > ,
   30 | pub defaults : TrackDefaults ,
   32 | pub
```


### 46 normalised tokens: `crates/cenc/src/movie.rs` lines 39 to 56, and kopuz-1d8b323 `daemon/db/src/lib.rs` lines 234 to 246

```text
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   51 | pub struct SampleEntry {
   53 | pub at : usize ,
   56 | pub format :
```


### 45 normalised tokens: `crates/cenc/src/movie.rs` lines 32 to 46, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 173 to 182

```text
   32 | Group > ,
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
```


### 44 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 106, and kopuz-1d8b323 `shared/api/src/library.rs` lines 182 to 190

```text
   94 | Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  104 | pub struct Pssh {
  106 | pub system_id :
```


### 44 normalised tokens: `crates/cenc/src/tests/build.rs` lines 375 to 385, and kopuz-1d8b323 `shared/api/src/catalog.rs` lines 88 to 99

```text
  375 | ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize
```


### 43 normalised tokens: `crates/cenc/src/table.rs` lines 95 to 101, and kopuz-1d8b323 `apps/kopuz/settings_appearance/src/lib.rs` lines 57 to 63

```text
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec <
```


### 42 normalised tokens: `crates/cenc/src/tests/build.rs` lines 219 to 233, and kopuz-1d8b323 `apps/kopuz/store/src/jobs.rs` lines 18 to 29

```text
  219 | ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool ,
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec
```


### 42 normalised tokens: `crates/cenc/src/tests/build.rs` lines 304 to 310, and kopuz-1d8b323 `apps/legacy/components/src/settings/remote_folders.rs` lines 13 to 18

```text
  304 | # [ derive ( Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
```


### 41 normalised tokens: `crates/cenc/src/lib.rs` lines 14 to 27, and kopuz-1d8b323 `shared/proto/src/convert/mod.rs` lines 14 to 27

```text
   14 | mod boxes ;
   15 | mod claims ;
   16 | mod clear ;
   17 | mod error ;
   18 | mod fragment ;
   19 | mod indexer ;
   20 | mod movie ;
   21 | mod stream ;
   22 | mod table ;
   24 | # [ cfg ( test ) ]
   25 | mod tests ;
   27 | pub use clear ::
```


### 41 normalised tokens: `crates/cenc/src/tests/build.rs` lines 375 to 383, and kopuz-1d8b323 `shared/api/src/library.rs` lines 51 to 59

```text
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 40 normalised tokens: `crates/cenc/src/indexer.rs` lines 43 to 49, and kopuz-1d8b323 `apps/kopuz/palette/src/lib.rs` lines 195 to 201

```text
   43 | end ,
   44 | table : SampleTable :: default ( ) ,
   45 | mdats : Vec :: new ( ) ,
   46 | descriptions : HashMap :: new ( ) ,
   47 | claims : Claims :: default ( ) ,
   48 | pending_needs : None ,
   49 | steps :
```


### 40 normalised tokens: `crates/cenc/src/movie.rs` lines 5 to 17, and kopuz-1d8b323 `daemon/server/src/ytmusic/accounts.rs` lines 7 to 18

```text
    5 | ;
    8 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
    9 | pub struct Movie {
   11 | pub tracks : Vec < Track > ,
   13 | pub pssh : Vec < Pssh > ,
   15 | pub end : usize ,
   17 | pub
```


### 40 normalised tokens: `crates/cenc/src/stream.rs` lines 326 to 330, and kopuz-1d8b323 `daemon/discord-presence/src/lib.rs` lines 105 to 109

```text
  326 | fn lock ( & self ) -> MutexGuard < '_ , State > {
  327 | self . state . lock ( ) . unwrap_or_else ( | e | e . into_inner ( ) )
  328 | }
  330 | fn decrypt_all ( & self ,
```


### 40 normalised tokens: `crates/cenc/src/table.rs` lines 80 to 94, and kopuz-1d8b323 `daemon/kopuzd/src/updates.rs` lines 48 to 56

```text
   80 | Debug , Default , PartialEq , Eq ) ]
   81 | struct Fields {
   82 | size : u32 ,
   83 | duration : u32 ,
   85 | iv_at : u32 ,
   87 | runs_at : u32 ,
   88 | }
   94 | # [ derive ( Clone , Debug , Default , PartialEq
```


### 40 normalised tokens: `crates/cenc/src/table.rs` lines 94 to 98, and kopuz-1d8b323 `apps/kopuz/store/src/playing.rs` lines 6 to 10

```text
   94 | # [ derive ( Clone , Debug , Default , PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
```


### 39 normalised tokens: `crates/cenc/src/fragment.rs` lines 184 to 192, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 175 to 185

```text
  184 | ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool ,
  190 | }
  192 | fn tfhd ( data :
```


### 39 normalised tokens: `crates/cenc/src/fragment.rs` lines 505 to 512, and kopuz-1d8b323 `apps/kopuz/store/src/jobs.rs` lines 185 to 192

```text
  505 | ,
  506 | scheme : protection . scheme ,
  507 | pattern : group . pattern ,
  508 | iv_size : group . iv_size ,
  509 | key_id : group . key_id ,
  510 | constant_iv : group . constant_iv ,
  511 | constant_iv_size : group . constant_iv_size ,
  512 | } )
```


### 39 normalised tokens: `crates/cenc/src/movie.rs` lines 32 to 41, and kopuz-1d8b323 `shared/api/src/playlists.rs` lines 26 to 33

```text
   32 | groups : Vec < Group > ,
   33 | }
   36 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
```


### 39 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 103, and kopuz-1d8b323 `shared/api/src/playlists.rs` lines 30 to 36

```text
   94 | Copy , Debug , Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 39 normalised tokens: `crates/cenc/src/tests/build.rs` lines 216 to 223, and kopuz-1d8b323 `daemon/server/src/ytmusic/discover.rs` lines 128 to 133

```text
  216 | Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool
```


### 38 normalised tokens: `crates/cenc/src/fragment.rs` lines 183 to 189, and kopuz-1d8b323 `apps/kopuz/components/src/link.rs` lines 55 to 61

```text
  183 | struct TrackHeader {
  184 | track_id : u32 ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 50 to 68, and kopuz-1d8b323 `daemon/core/src/integrations.rs` lines 286 to 294

```text
   50 | ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 611 to 614, and kopuz-1d8b323 `shared/config/src/edit.rs` lines 298 to 301

```text
  611 | Some ( p ) ,
  612 | SeekFrom :: End ( d ) => self . len . checked_add_signed ( d ) ,
  613 | SeekFrom :: Current ( d ) => self . pos . checked_add_signed ( d ) ,
  614 | }
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 619 to 626, and kopuz-1d8b323 `daemon/server/src/stream/range_source.rs` lines 325 to 332

```text
  619 | }
  621 | impl Drop for AudioStream {
  622 | fn drop ( & mut self ) {
  623 | self . shared . lock ( ) . abandoned = true ;
  624 | self . shared . changed . notify_all ( ) ;
  625 | }
  626 | }
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 223 to 234, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 364 to 374

```text
  223 | ,
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec < u8 > ,
  234 | pub subsamples : Vec <
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 254 to 262, and kopuz-1d8b323 `shared/utils/src/color.rs` lines 7 to 13

```text
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool ,
  258 | }
  260 | impl RunSpec {
  262 | pub fn of ( count : u32 , size : u32 , iv_size : usize ) -> Self {
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 372 to 379, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/wayland/client.rs` lines 307 to 312

```text
  372 | Clone , Debug , Default ) ]
  373 | pub struct FileSpec {
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8
```


### 37 normalised tokens: `crates/cenc/src/fragment.rs` lines 184 to 190, and kopuz-1d8b323 `apps/kopuz/components/src/selection.rs` lines 53 to 59

```text
  184 | track_id : u32 ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool ,
  190 | }
```


### 37 normalised tokens: `crates/cenc/src/movie.rs` lines 81 to 94, and kopuz-1d8b323 `apps/legacy/components/src/layout/showcase.rs` lines 7 to 15

```text
   81 | Copy , Debug , Hash , PartialEq , Eq ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
   91 | }
   94 | # [ derive ( Clone , Copy , Debug , Default , Hash
```


### 37 normalised tokens: `crates/cenc/src/stream.rs` lines 46 to 61, and kopuz-1d8b323 `apps/kopuz/metadata_dialog/src/lib.rs` lines 67 to 75

```text
   46 | ,
   47 | samples : Samples ,
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool
```


### 37 normalised tokens: `crates/cenc/src/table.rs` lines 94 to 99, and kopuz-1d8b323 `apps/legacy/hooks/src/ui_prefs.rs` lines 19 to 25

```text
   94 | PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 182 to 190, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 365 to 374

```text
  182 | }
  184 | # [ derive ( Clone , Debug ) ]
  185 | pub struct TrackSpec {
  186 | pub id : u32 ,
  187 | pub timescale : u32 ,
  188 | pub entries : Vec < Entry > ,
  190 | pub trex : Option <
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 213 to 221, and kopuz-1d8b323 `apps/legacy/components/src/common/controls/dots_menu.rs` lines 60 to 67

```text
  213 | }
  214 | }
  216 | # [ derive ( Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 218 to 227, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 786 to 793

```text
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool ,
  224 | }
  227 | # [ derive ( Clone , Debug
```


### 36 normalised tokens: `crates/cenc/src/lib.rs` lines 27 to 30, and kopuz-1d8b323 `apps/kopuz/components/src/lib.rs` lines 147 to 151

```text
   27 | last_sample_duration , make_clear , write_key_ids } ;
   28 | pub use error :: Error ;
   29 | pub use indexer :: Indexer ;
   30 | pub use movie :: { Movie , Pattern , Protection , Pssh , SampleEntry , Scheme ,
```


### 36 normalised tokens: `crates/cenc/src/stream.rs` lines 50 to 63, and kopuz-1d8b323 `daemon/core/src/spotify.rs` lines 45 to 56

```text
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 113 to 121, and kopuz-1d8b323 `daemon/server/src/applemusic/cenc.rs` lines 324 to 336

```text
  113 | }
  116 | # [ derive ( Clone , Debug ) ]
  117 | pub struct Seig {
  118 | pub protected : bool ,
  119 | pub iv_size : u8 ,
  120 | pub key_id : [ u8 ; 16 ] ,
  121 | pub pattern :
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 249 to 257, and kopuz-1d8b323 `daemon/core/src/external.rs` lines 45 to 53

```text
  249 | Clone , Debug ) ]
  250 | pub struct RunSpec {
  251 | pub samples : Vec < SampleSpec > ,
  253 | pub data_offset : bool ,
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool
```


### 36 normalised tokens: `crates/cenc/src/tests/build.rs` lines 291 to 304, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 381 to 392

```text
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
  302 | }
  304 | # [ derive ( Clone , Debug
```


### 36 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 18 to 20, and kopuz-1d8b323 `daemon/kopuzd/tests/contract.rs` lines 2057 to 2059

```text
   18 | let mut starts : Vec < usize > = table . iter ( ) . map ( | s | s . fragment ) . collect ( ) ;
   19 | starts . dedup ( ) ;
   20 | starts
```


### 35 normalised tokens: `crates/cenc/src/boxes.rs` lines 335 to 335, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/x11/clipboard.rs` lines 367 to 368

```text
  335 | 4 ) . map ( | b | u32 :: from_be_bytes ( [ b [ 0 ] , b [ 1 ] , b [ 2 ] , b [ 3 ] ] ) )
```


### 35 normalised tokens: `crates/cenc/src/clear.rs` lines 117 to 121, and kopuz-1d8b323 `apps/kopuz/store/src/downloads.rs` lines 200 to 205

```text
  117 | {
  118 | let Some ( entry ) = track . entries . iter ( ) . find ( | entry | entry . at == entry_box . at ) else {
  119 | continue ;
  120 | } ;
  121 | if
```


### 35 normalised tokens: `crates/cenc/src/lib.rs` lines 30 to 32, and kopuz-1d8b323 `apps/kopuz/theme/src/lib.rs` lines 18 to 20

```text
   30 | Pattern , Protection , Pssh , SampleEntry , Scheme , Track , TrackDefaults } ;
   31 | pub use stream :: { AudioStream , Decrypt } ;
   32 | pub use table :: { Sample , SampleTable } ;
```


### 35 normalised tokens: `crates/cenc/src/stream.rs` lines 53 to 68, and kopuz-1d8b323 `apps/kopuz/components/src/seek_bar.rs` lines 99 to 106

```text
   53 | ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize ,
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 44 to 51, and kopuz-1d8b323 `daemon/server/src/nextcloud/tree.rs` lines 43 to 49

```text
   44 | pub ( crate ) struct Description {
   45 | pub entry : u32 ,
   46 | pub protected : bool ,
   47 | pub scheme : Scheme ,
   48 | pub pattern : Pattern ,
   50 | pub iv_size : u8 ,
   51 | pub key_id :
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 174 to 181, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 768 to 775

```text
  174 | }
  177 | pub ( crate ) fn fragments ( & self ) -> & [ Fragment ] {
  178 | & self . fragments
  179 | }
  181 | pub ( crate ) fn mark ( & self ) -> Mark {
```


### 35 normalised tokens: `crates/cenc/src/table.rs` lines 298 to 302, and kopuz-1d8b323 `apps/kopuz/components/src/selection.rs` lines 300 to 304

```text
  298 | fn shrink_to_fit ( & mut self ) {
  299 | self . offsets . shrink_to_fit ( ) ;
  300 | self . fields . shrink_to_fit ( ) ;
  301 | self . description_of . shrink_to_fit ( ) ;
  302 | self . run_counts
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 290 to 304, and kopuz-1d8b323 `apps/kopuz/components/src/track_meta.rs` lines 211 to 228

```text
  290 | }
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
  302 | }
  304 | # [ derive ( Clone
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 302 to 309, and kopuz-1d8b323 `daemon/server/src/applemusic/cenc.rs` lines 343 to 354

```text
  302 | }
  304 | # [ derive ( Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 310 to 317, and kopuz-1d8b323 `daemon/server/src/download_queue.rs` lines 32 to 37

```text
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux ,
  317 | pub sbgp : Vec
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 400 to 409, and kopuz-1d8b323 `daemon/server/src/ytmusic/browse/search.rs` lines 112 to 120

```text
  400 | Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
  409 | pub
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 175 to 177, and kopuz-1d8b323 `apps/kopuz/now_playing/src/lyrics_view.rs` lines 735 to 738

```text
  175 | ) ;
  176 | let entries : Vec < u32 > = table . iter ( ) . map ( | s | s . entry ) . collect ( ) ;
  177 | assert_eq ! ( entries ,
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 198 to 200, and kopuz-1d8b323 `apps/kopuz/now_playing/src/lyrics_view.rs` lines 735 to 738

```text
  198 | ) ;
  199 | let durations : Vec < u32 > = table . iter ( ) . map ( | s | s . duration ) . collect ( ) ;
  200 | assert_eq ! ( durations ,
```


### 35 normalised tokens: `crates/cenc/src/tests/other.rs` lines 94 to 100, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 1233 to 1239

```text
   94 | ) ;
   95 | assert_eq ! ( source . to_string ( ) , error . to_string ( ) ) ;
   96 | }
   98 | # [ test ]
   99 | fn an_empty_file_and_a_file_of_only_an_ftyp ( ) {
  100 | assert_eq ! ( Movie :: read (
```


### 34 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 11, and kopuz-1d8b323 `apps/kopuz/components/src/folders.rs` lines 10 to 15

```text
    8 | Kind , Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track
```


### 34 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 11, and kopuz-1d8b323 `apps/kopuz/components/src/states.rs` lines 10 to 15

```text
    8 | Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track } ;
```


### 34 normalised tokens: `crates/cenc/src/error.rs` lines 22 to 28, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 52 to 58

```text
   22 | ) ,
   23 | }
   24 | }
   25 | }
   27 | impl fmt :: Display for Error {
   28 | fn fmt ( & self , f : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
```


### 34 normalised tokens: `crates/cenc/src/fragment.rs` lines 255 to 261, and kopuz-1d8b323 `apps/cli/src/lib.rs` lines 456 to 461

```text
  255 | let track = cx
  256 | . tracks
  257 | . iter ( )
  258 | . find ( | track | track . id == head . track_id )
  259 | . ok_or_else ( || {
  260 | Error :: new (
  261 | tfhd_box
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 200 to 214, and kopuz-1d8b323 `apps/kopuz/primitives/src/focus.rs` lines 54 to 64

```text
  200 | }
  203 | pub fn len ( & self ) -> u64 {
  204 | self . len
  205 | }
  208 | pub fn is_empty ( & self ) -> bool {
  209 | self . len == 0
  210 | }
  214 | pub fn with_samples
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 313 to 319, and kopuz-1d8b323 `apps/legacy/components/src/layout/normal/showcase.rs` lines 249 to 255

```text
  313 | ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs ,
  319 | protected : sample . protected
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 608 to 611, and kopuz-1d8b323 `daemon/server/src/audio_cd/mod.rs` lines 156 to 159

```text
  608 | {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
  610 | let pos = match to {
  611 | SeekFrom :: Start ( p ) => Some
```


### 34 normalised tokens: `crates/cenc/src/table.rs` lines 34 to 44, and kopuz-1d8b323 `shared/api/src/player.rs` lines 57 to 63

```text
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
   44 | pub
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 339 to 347, and kopuz-1d8b323 `apps/legacy/components/src/common/controls/dots_menu.rs` lines 63 to 68

```text
  339 | Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1029 to 1034, and kopuz-1d8b323 `daemon/db/src/backend/rows.rs` lines 104 to 109

```text
 1029 | Resolved {
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv . clone (
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1030 to 1034, and kopuz-1d8b323 `daemon/db/src/backend/migrations.rs` lines 1402 to 1406

```text
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv . clone ( ) ,
```


### 33 normalised tokens: `crates/cenc/src/clear.rs` lines 7 to 11, and kopuz-1d8b323 `apps/kopuz/discover/src/landing.rs` lines 20 to 25

```text
    7 | boxes :: {
    8 | Header , Kind , Lenient , lenient_at , lenient_kind , lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate ::
```


### 33 normalised tokens: `crates/cenc/src/clear.rs` lines 8 to 13, and kopuz-1d8b323 `apps/kopuz/components/src/pick_list.rs` lines 12 to 18

```text
    8 | lenient_u32 , retype , slice , write_u32 ,
    9 | } ;
   10 | use crate :: movie :: fixed_fields ;
   11 | use crate :: { Movie , SampleTable , Track } ;
   13 | const FREE : Kind =
```


### 33 normalised tokens: `crates/cenc/src/fragment.rs` lines 108 to 117, and kopuz-1d8b323 `daemon/core/src/session/mod.rs` lines 2588 to 2595

```text
  108 | ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option < Header > ,
  114 | groups : Vec < Group > ,
  115 | }
  117 | fn locate (
```


### 33 normalised tokens: `crates/cenc/src/indexer.rs` lines 18 to 27, and kopuz-1d8b323 `daemon/server/src/ytmusic/tracking.rs` lines 44 to 50

```text
   18 | # [ derive ( Debug ) ]
   19 | pub struct Indexer {
   20 | tracks : Vec < Track > ,
   22 | position : usize ,
   24 | scanned_to : usize ,
   25 | table : SampleTable ,
   27 | mdats : Vec
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 46 to 57, and kopuz-1d8b323 `apps/kopuz/components/src/settings.rs` lines 566 to 571

```text
   46 | data : Vec < u8 > ,
   47 | samples : Samples ,
   50 | decrypted : Vec < bool > ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 271 to 274, and kopuz-1d8b323 `daemon/server/src/ytmusic/mix.rs` lines 52 to 55

```text
  271 | fn table ( & self ) -> Option < & SampleTable > {
  272 | match self {
  273 | Self :: Whole ( table ) => Some ( table ) ,
  274 | Self :: Arriving ( Some
```


### 33 normalised tokens: `crates/cenc/src/table.rs` lines 99 to 103, and kopuz-1d8b323 `apps/kopuz/components/src/track_header.rs` lines 78 to 82

```text
   99 | ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec < ( u32 , u32 ) > ,
  102 | descriptions : Vec < Description > ,
  103 | fragments : Vec < Fragment > ,
```


### 33 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 187, and kopuz-1d8b323 `daemon/core/src/session/state.rs` lines 157 to 161

```text
  183 | ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments : self . fragments . len ( ) ,
  187 | }
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 340 to 349, and kopuz-1d8b323 `daemon/core/src/session/mod.rs` lines 46 to 52

```text
  340 | struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 370 to 377, and kopuz-1d8b323 `daemon/core/src/external.rs` lines 42 to 49

```text
  370 | }
  372 | # [ derive ( Clone , Debug , Default ) ]
  373 | pub struct FileSpec {
  375 | pub leading : Vec < u8 > ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1267 to 1270, and kopuz-1d8b323 `apps/legacy/hooks/src/use_db_queries.rs` lines 68 to 71

```text
 1267 | corpus ( )
 1268 | . into_iter ( )
 1269 | . find ( | ( found , _ ) | * found == name )
 1270 | . map ( | ( _ , spec ) | spec
```


### 33 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 33 to 36, and kopuz-1d8b323 `daemon/core/src/persistence.rs` lines 221 to 224

```text
   33 | clear_movie
   34 | . tracks
   35 | . iter ( )
   36 | . all ( | t | t . entries . iter ( ) . all ( | e | e . protection . is_none ( )
```


### 33 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 18 to 22, and kopuz-1d8b323 `daemon/core/src/library/mod.rs` lines 1205 to 1209

```text
   18 | table . iter ( ) . map ( | s | s . fragment ) . collect ( ) ;
   19 | starts . dedup ( ) ;
   20 | starts
   21 | } ;
   22 | assert_eq ! ( fragments
```


### 33 normalised tokens: `crates/cenc/src/tests/other.rs` lines 94 to 99, and kopuz-1d8b323 `apps/kopuz/store/src/notices.rs` lines 257 to 262

```text
   94 | error . clone ( ) ) ;
   95 | assert_eq ! ( source . to_string ( ) , error . to_string ( ) ) ;
   96 | }
   98 | # [ test ]
   99 | fn an_empty_file_and_a_file_of_only_an_ftyp ( ) {
```


### 32 normalised tokens: `crates/cenc/examples/index_cost.rs` lines 15 to 18, and kopuz-1d8b323 `daemon/core/src/offline.rs` lines 3 to 6

```text
   15 | ;
   16 | use std :: hint :: black_box ;
   17 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
   18 | use std :: time :: { Duration , Instant } ;
```


### 32 normalised tokens: `crates/cenc/src/error.rs` lines 24 to 29, and kopuz-1d8b323 `apps/kopuz/store/src/target.rs` lines 66 to 71

```text
   24 | }
   25 | }
   27 | impl fmt :: Display for Error {
   28 | fn fmt ( & self , f : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
   29 | write
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 100 to 108, and kopuz-1d8b323 `daemon/player/src/engine/worker.rs` lines 111 to 119

```text
  100 | }
  104 | # [ derive ( Default ) ]
  105 | struct TrafBoxes {
  106 | tfhd : Option < Header > ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 390 to 393, and kopuz-1d8b323 `apps/kopuz/components/src/karaoke.rs` lines 118 to 119

```text
  390 | let default_duration = head
  391 | . default_duration
  392 | . unwrap_or ( track . defaults . sample_duration ) ;
  393 | let default_size = head . default_size . unwrap_or ( track . defaults . sample_size ) ;
```


### 32 normalised tokens: `crates/cenc/src/lib.rs` lines 30 to 40, and kopuz-1d8b323 `apps/kopuz/store/src/lib.rs` lines 49 to 52

```text
   30 | Pssh , SampleEntry , Scheme , Track , TrackDefaults } ;
   31 | pub use stream :: { AudioStream , Decrypt } ;
   32 | pub use table :: { Sample , SampleTable } ;
   40 | pub
```


### 32 normalised tokens: `crates/cenc/src/movie.rs` lines 100 to 108, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/mediadrm.rs` lines 30 to 41

```text
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  104 | pub struct Pssh {
  106 | pub system_id : [ u8 ; 16 ] ,
  108 | pub key_ids : Vec <
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 32 to 43, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 409 to 416

```text
   32 | ,
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 88 to 97, and kopuz-1d8b323 `apps/kopuz/components/src/selection.rs` lines 185 to 192

```text
   88 | }
   94 | # [ derive ( Clone , Debug , Default , PartialEq , Eq ) ]
   95 | pub struct SampleTable {
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 315 to 323, and kopuz-1d8b323 `apps/kopuz/library/src/filters.rs` lines 17 to 24

```text
  315 | ,
  317 | pub sbgp : Vec < ( u32 , u32 ) > ,
  319 | pub sgpd : Vec < Seig > ,
  320 | }
  322 | impl TrafSpec {
  323 | pub fn new ( track_id : u32
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 880 to 880, and kopuz-1d8b323 `daemon/server/src/ytmusic/fixture_tests.rs` lines 191 to 191

```text
  880 | let samples : Vec < & SampleSpec > = traf . runs . iter ( ) . flat_map ( | run | & run . samples ) . collect ( ) ;
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 935 to 941, and kopuz-1d8b323 `apps/legacy/kopuz/src/updates.rs` lines 216 to 223

```text
  935 | fn expected (
  936 | spec : & FragmentSpec ,
  937 | movie : & MovieSpec ,
  938 | moof_at : usize ,
  939 | layouts : & [ TrafLayout ] ,
  940 | ) -> Vec < Expected > {
  941 | let
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1314 to 1314, and kopuz-1d8b323 `daemon/server/src/applemusic/cenc.rs` lines 4 to 4

```text
 1314 | u32 :: from_be_bytes ( [ bytes [ at ] , bytes [ at + 1 ] , bytes [ at + 2 ] , bytes [ at + 3 ] ] )
```


### 32 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 17 to 18, and kopuz-1d8b323 `apps/kopuz/page_albums/src/detail.rs` lines 553 to 556

```text
   17 | let fragments : Vec < usize > = {
   18 | let mut starts : Vec < usize > = table . iter ( ) . map ( | s | s . fragment
```


### 32 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 177 to 179, and kopuz-1d8b323 `daemon/db/tests/persistence.rs` lines 148 to 151

```text
  177 | ] ) ;
  178 | assert_eq ! ( movie . pssh . len ( ) , 1 ) ;
  179 | assert_eq ! ( movie . pssh [ 0 ] . key_ids , vec ! [
```


### 31 normalised tokens: `crates/cenc/src/boxes.rs` lines 188 to 199, and kopuz-1d8b323 `apps/kopuz/store/src/setup.rs` lines 67 to 76

```text
  188 | body ,
  189 | }
  190 | }
  193 | pub fn at ( & self ) -> usize {
  194 | self . at
  195 | }
  198 | pub fn left ( & self ) -> usize {
  199 | self . header
```


### 31 normalised tokens: `crates/cenc/src/fragment.rs` lines 8 to 13, and kopuz-1d8b323 `apps/kopuz/page_home/src/shelf.rs` lines 5 to 10

```text
    8 | ;
    9 | use crate :: movie :: { Group , SampleEntry , Track , sgpd } ;
   10 | use crate :: table :: { Description , SampleTable } ;
   13 | pub ( crate )
```


### 31 normalised tokens: `crates/cenc/src/indexer.rs` lines 7 to 10, and kopuz-1d8b323 `apps/legacy/components/src/track/actions.rs` lines 5 to 8

```text
    7 | , Fault } ;
    8 | use crate :: movie :: { Movie , Track } ;
    9 | use crate :: table :: { Description , Fragment , SampleTable } ;
   10 | use crate ::
```


### 31 normalised tokens: `crates/cenc/src/indexer.rs` lines 10 to 25, and kopuz-1d8b323 `apps/kopuz/topbar/src/switcher.rs` lines 34 to 42

```text
   10 | ;
   18 | # [ derive ( Debug ) ]
   19 | pub struct Indexer {
   20 | tracks : Vec < Track > ,
   22 | position : usize ,
   24 | scanned_to : usize ,
   25 | table : SampleTable ,
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 77 to 90, and kopuz-1d8b323 `apps/kopuz/store/src/target.rs` lines 118 to 127

```text
   77 | ,
   78 | }
   81 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 84 to 95, and kopuz-1d8b323 `shared/config/src/lib.rs` lines 122 to 130

```text
   84 | Cenc ,
   86 | Cens ,
   88 | Cbc1 ,
   90 | Cbcs ,
   91 | }
   94 | # [ derive ( Clone , Copy , Debug , Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 530 to 535, and kopuz-1d8b323 `daemon/server/src/smb/stream.rs` lines 40 to 45

```text
  530 | }
  531 | }
  533 | impl Read for AudioStream {
  534 | fn read ( & mut self , buf : & mut [ u8 ] ) -> io :: Result < usize > {
  535 | if
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 743 to 746, and kopuz-1d8b323 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  743 | ) ;
  744 | let mut out = Vec :: new ( ) ;
  745 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  746 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 893 to 896, and kopuz-1d8b323 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  893 | ) ;
  894 | let mut out = Vec :: new ( ) ;
  895 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  896 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 911 to 914, and kopuz-1d8b323 `daemon/server/src/applemusic/progressive.rs` lines 688 to 691

```text
  911 | ) ;
  912 | let mut out = Vec :: new ( ) ;
  913 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  914 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 74 to 80, and kopuz-1d8b323 `apps/kopuz/store/src/downloads.rs` lines 42 to 48

```text
   74 | struct Fragment {
   75 | pub start : usize ,
   76 | pub first_sample : usize ,
   77 | }
   80 | # [ derive ( Clone , Copy , Debug , Default , PartialEq , Eq ) ]
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 80 to 88, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/wayland/client.rs` lines 117 to 123

```text
   80 | Copy , Debug , Default , PartialEq , Eq ) ]
   81 | struct Fields {
   82 | size : u32 ,
   83 | duration : u32 ,
   85 | iv_at : u32 ,
   87 | runs_at : u32 ,
   88 | }
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 288 to 301, and kopuz-1d8b323 `apps/kopuz/library/src/filters.rs` lines 162 to 173

```text
  288 | }
  289 | flags
  290 | }
  291 | }
  294 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  295 | pub enum Aux {
  296 | Senc ,
  298 | SaizSaio ,
  300 | EmptySenc ,
  301 | None ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 310 to 317, and kopuz-1d8b323 `daemon/db/src/lib.rs` lines 56 to 62

```text
  310 | u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux ,
  317 | pub sbgp : Vec <
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 341 to 349, and kopuz-1d8b323 `apps/kopuz/page_home/src/data.rs` lines 35 to 40

```text
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 396 to 403, and kopuz-1d8b323 `apps/legacy/hooks/src/use_db_queries.rs` lines 31 to 39

```text
  396 | ,
  397 | }
  400 | # [ derive ( Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 402 to 409, and kopuz-1d8b323 `apps/legacy/components/src/playback/controls.rs` lines 121 to 126

```text
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
  409 | pub fn build ( spec :
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 416 to 420, and kopuz-1d8b323 `apps/legacy/components/src/navigation/controller.rs` lines 110 to 114

```text
  416 | cleared ) ;
  417 | bytes . extend ( built . bytes ) ;
  418 | samples . extend ( built . samples ) ;
  419 | fragments . push ( built . moof_at ) ;
  420 | }
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1268 to 1270, and kopuz-1d8b323 `shared/config/src/store.rs` lines 269 to 271

```text
 1268 | . into_iter ( )
 1269 | . find ( | ( found , _ ) | * found == name )
 1270 | . map ( | ( _ , spec ) | spec )
```


### 31 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 39 to 40, and kopuz-1d8b323 `daemon/core/src/prefs.rs` lines 102 to 106

```text
   39 | ) > = table . iter ( ) . map ( | s | ( s . offset , s . size ) ) . collect ( ) ;
   40 | let clear_places
```


### 31 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 205 to 207, and kopuz-1d8b323 `apps/legacy/pages/src/server/browse.rs` lines 620 to 622

```text
  205 | ) ;
  206 | assert_eq ! ( movie . pssh . len ( ) , 2 ) ;
  207 | assert ! ( movie . pssh [ 0 ] . key_ids . is_empty ( )
```


### 31 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 206 to 208, and kopuz-1d8b323 `daemon/core/src/sources/removable.rs` lines 298 to 300

```text
  206 | ) ;
  207 | assert ! ( movie . pssh [ 0 ] . key_ids . is_empty ( ) ) ;
  208 | assert_eq ! ( movie . pssh [ 0 ] . data ,
```


### 31 normalised tokens: `crates/cenc/src/tests/other.rs` lines 154 to 156, and kopuz-1d8b323 `daemon/core/src/queue_model.rs` lines 805 to 808

```text
  154 | ) ;
  155 | assert_eq ! ( table . len ( ) , 3 ) ;
  156 | assert ! ( table . iter ( ) . all ( | s | s . offset
```


### 31 normalised tokens: `crates/cenc/tests/allocation.rs` lines 5 to 7, and kopuz-1d8b323 `apps/kopuz/page_settings/src/lib.rs` lines 5 to 7

```text
    5 | GlobalAlloc , Layout , System } ;
    6 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    7 | use std :: sync :: { Mutex , MutexGuard } ;
```


### 31 normalised tokens: `crates/cenc/tests/allocation.rs` lines 121 to 123, and kopuz-1d8b323 `apps/kopuz/components/src/schema_form.rs` lines 340 to 342

```text
  121 | read ( & file ) . unwrap ( ) . unwrap ( ) ;
  122 | let mut indexer = cenc :: Indexer :: new ( & movie ) ;
  123 | for end in
```


### 30 normalised tokens: `crates/cenc/examples/index_cost.rs` lines 15 to 18, and kopuz-1d8b323 `daemon/player/src/engine/worker.rs` lines 10 to 13

```text
   15 | Layout , System } ;
   16 | use std :: hint :: black_box ;
   17 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
   18 | use std :: time ::
```


### 30 normalised tokens: `crates/cenc/src/boxes.rs` lines 190 to 199, and kopuz-1d8b323 `apps/kopuz/store/src/query.rs` lines 155 to 164

```text
  190 | }
  193 | pub fn at ( & self ) -> usize {
  194 | self . at
  195 | }
  198 | pub fn left ( & self ) -> usize {
  199 | self . header . end
```


### 30 normalised tokens: `crates/cenc/src/boxes.rs` lines 286 to 292, and kopuz-1d8b323 `shared/api/src/artwork.rs` lines 60 to 66

```text
  286 | end : usize ,
  287 | }
  289 | impl Lenient {
  290 | pub fn new ( at : usize , end : usize ) -> Self {
  291 | Self { at , end }
  292 | }
```


### 30 normalised tokens: `crates/cenc/src/claims.rs` lines 97 to 103, and kopuz-1d8b323 `daemon/server/src/audio_cd/native.rs` lines 700 to 708

```text
   97 | ) ) ;
   98 | }
  100 | # [ test ]
  101 | fn a_fragment_given_back_owns_nothing_and_one_kept_owns_its_ranges ( ) {
  102 | let mut claims = Claims :: default ( ) ;
  103 | assert ! ( claims . claim (
```


### 30 normalised tokens: `crates/cenc/src/error.rs` lines 1 to 12, and kopuz-1d8b323 `daemon/radio/src/provider.rs` lines 7 to 12

```text
    1 | use std :: fmt ;
    7 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
    8 | pub struct Error {
   10 | pub offset : usize ,
   12 | pub kind :
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 151 to 154, and kopuz-1d8b323 `daemon/core/src/prefs.rs` lines 18 to 20

```text
  151 | Header , parent : Header ) -> Result < ( ) , Error > {
  152 | if slot . is_some ( ) {
  153 | return Err ( Error :: new (
  154 | parent
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 476 to 481, and kopuz-1d8b323 `apps/legacy/components/src/layout/vaxry/showcase.rs` lines 245 to 250

```text
  476 | protected : true ,
  477 | scheme : protection . scheme ,
  478 | pattern : protection . pattern ,
  479 | iv_size : protection . iv_size ,
  480 | key_id : protection . key_id ,
  481 | constant_iv ,
```


### 30 normalised tokens: `crates/cenc/src/lib.rs` lines 28 to 30, and kopuz-1d8b323 `daemon/player/src/engine/mod.rs` lines 16 to 19

```text
   28 | use error :: Error ;
   29 | pub use indexer :: Indexer ;
   30 | pub use movie :: { Movie , Pattern , Protection , Pssh , SampleEntry , Scheme , Track ,
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 62 to 71, and kopuz-1d8b323 `daemon/db/src/lib.rs` lines 223 to 232

```text
   62 | children_at : usize ,
   63 | }
   66 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   67 | pub struct Protection {
   69 | pub scheme : Scheme ,
   71 | pub key_id :
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 178 to 181, and kopuz-1d8b323 `daemon/server/src/source/apple_music.rs` lines 270 to 274

```text
  178 | ) -> Result < Movie , Error > {
  179 | let mut tracks = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 179 to 181, and kopuz-1d8b323 `daemon/server/src/nextcloud/mod.rs` lines 186 to 188

```text
  179 | let mut tracks = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let mut defaults = Vec :: new ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 179 to 182, and kopuz-1d8b323 `daemon/server/src/ytmusic/discover.rs` lines 530 to 533

```text
  179 | = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let mut defaults = Vec :: new ( ) ;
  182 | let mut children
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 21 to 25, and kopuz-1d8b323 `daemon/core/src/sources.rs` lines 9 to 13

```text
   21 | Read , Seek , SeekFrom } ;
   22 | use std :: sync :: { Arc , Condvar , Mutex , MutexGuard } ;
   23 | use std :: time :: Duration ;
   25 | use
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 94 to 101, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 141 to 147

```text
   94 | pub struct Feed {
   95 | shared : Arc < Shared > ,
   96 | }
   98 | impl AudioStream {
  100 | pub fn plain ( data : Vec < u8 > ) -> Self {
  101 | Self
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 604 to 609, and kopuz-1d8b323 `daemon/server/src/smb/stream.rs` lines 90 to 95

```text
  604 | Ok ( n )
  605 | }
  606 | }
  608 | impl Seek for AudioStream {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
```


### 30 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 186, and kopuz-1d8b323 `daemon/db/src/backend/cfg_store.rs` lines 492 to 495

```text
  183 | . len ( ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments : self . fragments
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 180 to 188, and kopuz-1d8b323 `apps/kopuz/store/src/notices.rs` lines 88 to 97

```text
  180 | )
  181 | }
  182 | }
  184 | # [ derive ( Clone , Debug ) ]
  185 | pub struct TrackSpec {
  186 | pub id : u32 ,
  187 | pub timescale : u32 ,
  188 | pub entries : Vec
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 192 to 202, and kopuz-1d8b323 `apps/legacy/hooks/src/downloads.rs` lines 24 to 31

```text
  192 | groups : Vec < Seig > ,
  194 | pub sgpd_version : u8 ,
  196 | pub version1_headers : bool ,
  198 | pub visual : bool ,
  199 | }
  201 | impl TrackSpec {
  202 | pub fn audio (
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 194 to 202, and kopuz-1d8b323 `shared/utils/src/color.rs` lines 7 to 13

```text
  194 | pub sgpd_version : u8 ,
  196 | pub version1_headers : bool ,
  198 | pub visual : bool ,
  199 | }
  201 | impl TrackSpec {
  202 | pub fn audio ( id : u32 , entries : Vec
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 229 to 234, and kopuz-1d8b323 `apps/kopuz/library/src/filters.rs` lines 15 to 18

```text
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec < u8 > ,
  234 | pub subsamples : Vec < ( u16 , u32 ) > ,
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 333 to 340, and kopuz-1d8b323 `daemon/core/src/offline_cache.rs` lines 136 to 143

```text
  333 | Vec :: new ( ) ,
  334 | sgpd : Vec :: new ( ) ,
  335 | }
  336 | }
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 337 to 343, and kopuz-1d8b323 `apps/legacy/hooks/src/use_db_queries.rs` lines 32 to 39

```text
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 343 to 353, and kopuz-1d8b323 `apps/legacy/hooks/src/downloads.rs` lines 24 to 31

```text
  343 | gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool ,
  350 | }
  352 | impl FragmentSpec {
  353 | pub fn new (
```


### 30 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 39 to 41, and kopuz-1d8b323 `apps/legacy/pages/src/settings/sections.rs` lines 126 to 130

```text
   39 | ) ) . collect ( ) ;
   40 | let clear_places : Vec < ( usize , usize ) > =
   41 | clear_table . iter ( ) . map ( | s |
```


### 30 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 49 to 52, and kopuz-1d8b323 `daemon/radio/src/browser.rs` lines 358 to 361

```text
   49 | ) ;
   50 | assert_eq ! ( movie . tracks . len ( ) , 1 ) ;
   51 | let track = & movie . tracks [ 0 ] ;
   52 | assert_eq ! (
```


### 30 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 169 to 174, and kopuz-1d8b323 `daemon/server/src/subsonic.rs` lines 984 to 994

```text
  169 | table . iter ( ) . all ( | s | s . key_id == OTHER_KEY ) ) ;
  170 | }
  172 | # [ test ]
  173 | fn several_sample_entries_for_spatial_audio ( ) {
  174 | let
```


### 46 normalised tokens: `crates/cenc/src/tests/build.rs` lines 397 to 409, and fmp4.rs@4522e68 `fmp4.rs` lines 333 to 351

```text
  397 | }
  400 | # [ derive ( Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
  409 | pub fn build ( spec : &
```


### 42 normalised tokens: `crates/cenc/src/stream.rs` lines 626 to 634, and fmp4.rs@4522e68 `fmp4.rs` lines 1113 to 1120

```text
  626 | }
  628 | # [ cfg ( test ) ]
  629 | mod tests {
  630 | use super :: * ;
  633 | fn boxed ( kind : & [ u8 ; 4 ] , body : & [ u8 ] ) -> Vec < u8 > {
  634 | let
```


### 41 normalised tokens: `crates/cenc/src/movie.rs` lines 178 to 182, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 151

```text
  178 | usize ) -> Result < Movie , Error > {
  179 | let mut tracks = Vec :: new ( ) ;
  180 | let mut pssh = Vec :: new ( ) ;
  181 | let mut defaults = Vec :: new ( ) ;
  182 | let
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 336 to 345, and fmp4.rs@4522e68 `fmp4.rs` lines 332 to 344

```text
  336 | }
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool
```


### 33 normalised tokens: `crates/cenc/src/tests/build.rs` lines 216 to 221, and fmp4.rs@4522e68 `fmp4.rs` lines 336 to 344

```text
  216 | Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1314 to 1314, and fmp4.rs@4522e68 `fmp4.rs` lines 10 to 10

```text
 1314 | u32 :: from_be_bytes ( [ bytes [ at ] , bytes [ at + 1 ] , bytes [ at + 2 ] , bytes [ at + 3 ] ] )
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 55 to 60, and fmp4.rs@4522e68 `fmp4.rs` lines 320 to 326

```text
   55 | ,
   56 | pub iv_size : u8 ,
   57 | pub key_id : [ u8 ; 16 ] ,
   59 | pub pattern : Option < ( u8 , u8 ) > ,
   60 | pub constant_iv : Option
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 165 to 166, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  165 | fn aux_is_cenc ( data : & [ u8 ] , header : Header ) -> Result < bool , Error > {
  166 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 175 to 176, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  175 | fn grouping_type ( data : & [ u8 ] , header : Header ) -> Result < Kind , Error > {
  176 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 192 to 193, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  192 | fn tfhd ( data : & [ u8 ] , header : Header ) -> Result < TrackHeader , Error > {
  193 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 234 to 235, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  234 | fn track_id ( data : & [ u8 ] , header : Header ) -> Result < u32 , Error > {
  235 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 268 to 269, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  268 | fn parse_timescale ( data : & [ u8 ] , header : Header ) -> Result < u32 , Error > {
  269 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 455 to 456, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  455 | fn schm ( data : & [ u8 ] , header : Header ) -> Result < Scheme , Error > {
  456 | let mut body = Body :: of (
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 698 to 699, and fmp4.rs@4522e68 `fmp4.rs` lines 146 to 147

```text
  698 | fn parse_pssh ( data : & [ u8 ] , header : Header ) -> Result < Pssh , Error > {
  699 | let mut body = Body :: of (
```


### 106 normalised tokens: `crates/cenc/src/stream.rs` lines 608 to 619, and tree `crates/downloads/src/sealed.rs` lines 208 to 219

```text
  608 | {
  609 | fn seek ( & mut self , to : SeekFrom ) -> io :: Result < u64 > {
  610 | let pos = match to {
  611 | SeekFrom :: Start ( p ) => Some ( p ) ,
  612 | SeekFrom :: End ( d ) => self . len . checked_add_signed ( d ) ,
  613 | SeekFrom :: Current ( d ) => self . pos . checked_add_signed ( d ) ,
  614 | } ;
  615 | self . pos =
  616 | pos . ok_or_else ( || io :: Error :: new ( io :: ErrorKind :: InvalidInput , "seek before start" ) ) ? ;
  617 | Ok ( self . pos )
  618 | }
  619 | }
```


### 55 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 17 to 26, and tree `crates/app-desktop/src/theme.rs` lines 60 to 69

```text
   17 | Expected {
   18 | offset : sample . offset ,
   19 | size : sample . size ,
   20 | duration : sample . duration ,
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples
```


### 53 normalised tokens: `crates/cenc/src/stream.rs` lines 309 to 318, and tree `crates/app-desktop/src/theme.rs` lines 60 to 69

```text
  309 | Sample {
  310 | offset : sample . offset ,
  311 | size : sample . size ,
  312 | duration : sample . duration ,
  313 | fragment : sample . fragment ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs
```


### 52 normalised tokens: `crates/cenc/src/stream.rs` lines 533 to 538, and tree `crates/downloads/src/sealed.rs` lines 193 to 198

```text
  533 | {
  534 | fn read ( & mut self , buf : & mut [ u8 ] ) -> io :: Result < usize > {
  535 | if self . pos >= self . len || buf . is_empty ( ) {
  536 | return Ok ( 0 ) ;
  537 | }
  538 | let start = self . pos
```


### 50 normalised tokens: `crates/cenc/src/stream.rs` lines 961 to 967, and tree `crates/opium-core/src/growing.rs` lines 140 to 146

```text
  961 | ) ;
  962 | feed . end ( None ) ;
  963 | } ) ;
  964 | let mut out = Vec :: new ( ) ;
  965 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  966 | feeding . join ( ) . unwrap ( ) ;
  967 | assert_eq ! ( out ,
```


### 48 normalised tokens: `crates/cenc/src/stream.rs` lines 765 to 769, and tree `crates/downloads/src/sealed.rs` lines 261 to 265

```text
  765 | ] ;
  766 | let n = stream . read ( & mut buf ) . unwrap ( ) ;
  767 | assert ! ( n > 0 ) ;
  768 | assert_eq ! ( & buf [ .. n ] , & clear [ inside .. inside + n ] ) ;
  769 | }
```


### 48 normalised tokens: `crates/cenc/src/tests/build.rs` lines 216 to 227, and tree `crates/player/src/lib.rs` lines 163 to 176

```text
  216 | Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8 > ,
  223 | pub largesize : bool ,
  224 | }
  227 | # [ derive ( Clone , Debug
```


### 47 normalised tokens: `crates/cenc/src/error.rs` lines 21 to 30, and tree `crates/opium-core/src/model.rs` lines 31 to 39

```text
   21 | kind ,
   22 | what : what . into ( ) ,
   23 | }
   24 | }
   25 | }
   27 | impl fmt :: Display for Error {
   28 | fn fmt ( & self , f : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
   29 | write ! (
   30 | f ,
```


### 47 normalised tokens: `crates/cenc/src/table.rs` lines 299 to 305, and tree `crates/app-desktop/src/app.rs` lines 1920 to 1928

```text
  299 | ;
  300 | self . fields . shrink_to_fit ( ) ;
  301 | self . description_of . shrink_to_fit ( ) ;
  302 | self . run_counts . shrink_to_fit ( ) ;
  303 | self . ivs . shrink_to_fit ( ) ;
  304 | self . runs . shrink_to_fit ( ) ;
  305 | self . descriptions . shrink_to_fit (
```


### 47 normalised tokens: `crates/cenc/src/table.rs` lines 300 to 306, and tree `crates/app-desktop/src/app.rs` lines 1920 to 1928

```text
  300 | ;
  301 | self . description_of . shrink_to_fit ( ) ;
  302 | self . run_counts . shrink_to_fit ( ) ;
  303 | self . ivs . shrink_to_fit ( ) ;
  304 | self . runs . shrink_to_fit ( ) ;
  305 | self . descriptions . shrink_to_fit ( ) ;
  306 | self . fragments . shrink_to_fit (
```


### 47 normalised tokens: `crates/cenc/src/tests/build.rs` lines 306 to 315, and tree `crates/local/src/scan.rs` lines 39 to 46

```text
  306 | ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux
```


### 47 normalised tokens: `crates/cenc/src/tests/build.rs` lines 379 to 390, and tree `crates/app-desktop/src/theme.rs` lines 100 to 111

```text
  379 | ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize ,
  386 | pub size : usize ,
  387 | pub duration : u32 ,
  388 | pub fragment : usize ,
  389 | pub entry : u32 ,
  390 | pub key_id :
```


### 47 normalised tokens: `crates/cenc/src/tests/build.rs` lines 453 to 458, and tree `crates/synced/src/tests.rs` lines 111 to 117

```text
  453 | ) ;
  454 | trex . extend ( index . to_be_bytes ( ) ) ;
  455 | trex . extend ( duration . to_be_bytes ( ) ) ;
  456 | trex . extend ( size . to_be_bytes ( ) ) ;
  457 | trex . extend ( flags . to_be_bytes ( ) ) ;
  458 | mvex
```


### 46 normalised tokens: `crates/cenc/src/movie.rs` lines 39 to 56, and tree `crates/app-desktop/src/theme.rs` lines 95 to 107

```text
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
   47 | }
   50 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   51 | pub struct SampleEntry {
   53 | pub at : usize ,
   56 | pub format :
```


### 46 normalised tokens: `crates/cenc/src/table.rs` lines 299 to 304, and tree `crates/app-desktop/src/app.rs` lines 1921 to 1928

```text
  299 | self . offsets . shrink_to_fit ( ) ;
  300 | self . fields . shrink_to_fit ( ) ;
  301 | self . description_of . shrink_to_fit ( ) ;
  302 | self . run_counts . shrink_to_fit ( ) ;
  303 | self . ivs . shrink_to_fit ( ) ;
  304 | self . runs . shrink_to_fit (
```


### 42 normalised tokens: `crates/cenc/src/stream.rs` lines 626 to 634, and tree `crates/player/src/gapless.rs` lines 184 to 191

```text
  626 | }
  628 | # [ cfg ( test ) ]
  629 | mod tests {
  630 | use super :: * ;
  633 | fn boxed ( kind : & [ u8 ; 4 ] , body : & [ u8 ] ) -> Vec < u8 > {
  634 | let
```


### 42 normalised tokens: `crates/cenc/src/table.rs` lines 43 to 51, and tree `crates/app-desktop/src/ui/menu.rs` lines 402 to 416

```text
   43 | Hash , PartialEq , Eq ) ]
   44 | pub ( crate ) struct Description {
   45 | pub entry : u32 ,
   46 | pub protected : bool ,
   47 | pub scheme : Scheme ,
   48 | pub pattern : Pattern ,
   50 | pub iv_size : u8 ,
   51 | pub key_id :
```


### 41 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 103, and tree `crates/synced/src/lib.rs` lines 32 to 40

```text
   94 | Clone , Copy , Debug , Default , Hash , PartialEq , Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 41 normalised tokens: `crates/cenc/src/table.rs` lines 301 to 306, and tree `crates/app-desktop/src/app.rs` lines 1920 to 1926

```text
  301 | ;
  302 | self . run_counts . shrink_to_fit ( ) ;
  303 | self . ivs . shrink_to_fit ( ) ;
  304 | self . runs . shrink_to_fit ( ) ;
  305 | self . descriptions . shrink_to_fit ( ) ;
  306 | self . fragments . shrink_to_fit ( ) ;
```


### 40 normalised tokens: `crates/cenc/src/fragment.rs` lines 505 to 513, and tree `crates/opium-ffi/src/lib.rs` lines 2041 to 2049

```text
  505 | ,
  506 | scheme : protection . scheme ,
  507 | pattern : group . pattern ,
  508 | iv_size : group . iv_size ,
  509 | key_id : group . key_id ,
  510 | constant_iv : group . constant_iv ,
  511 | constant_iv_size : group . constant_iv_size ,
  512 | } )
  513 | }
```


### 40 normalised tokens: `crates/cenc/src/indexer.rs` lines 43 to 49, and tree `crates/local/src/lib.rs` lines 647 to 653

```text
   43 | end ,
   44 | table : SampleTable :: default ( ) ,
   45 | mdats : Vec :: new ( ) ,
   46 | descriptions : HashMap :: new ( ) ,
   47 | claims : Claims :: default ( ) ,
   48 | pending_needs : None ,
   49 | steps :
```


### 40 normalised tokens: `crates/cenc/src/stream.rs` lines 947 to 954, and tree `crates/downloads/src/sealed.rs` lines 285 to 292

```text
  947 | ) ;
  948 | let mut out = Vec :: new ( ) ;
  949 | assert ! ( stream . read_to_end ( & mut out ) . is_err ( ) ) ;
  950 | }
  952 | # [ test ]
  953 | fn a_download_reads_as_it_comes ( ) {
  954 | let
```


### 40 normalised tokens: `crates/cenc/src/tests/build.rs` lines 304 to 312, and tree `crates/soundcloud/src/types.rs` lines 24 to 31

```text
  304 | Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool
```


### 40 normalised tokens: `crates/cenc/src/tests/build.rs` lines 339 to 349, and tree `crates/desktop/src/mpris.rs` lines 46 to 53

```text
  339 | Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool ,
  347 | pub mdat_first : bool ,
  349 | pub pssh : bool ,
```


### 40 normalised tokens: `crates/cenc/src/tests/build.rs` lines 396 to 406, and tree `crates/soundcloud/src/types.rs` lines 101 to 110

```text
  396 | ,
  397 | }
  400 | # [ derive ( Clone , Debug ) ]
  401 | pub struct Built {
  402 | pub bytes : Vec < u8 > ,
  403 | pub samples : Vec < Expected > ,
  405 | pub fragments : Vec < usize > ,
  406 | }
```


### 40 normalised tokens: `crates/cenc/src/tests/build.rs` lines 454 to 458, and tree `crates/synced/src/tests.rs` lines 111 to 116

```text
  454 | ) ;
  455 | trex . extend ( duration . to_be_bytes ( ) ) ;
  456 | trex . extend ( size . to_be_bytes ( ) ) ;
  457 | trex . extend ( flags . to_be_bytes ( ) ) ;
  458 | mvex . extend ( full
```


### 39 normalised tokens: `crates/cenc/src/movie.rs` lines 8 to 17, and tree `crates/desktop/src/mpris.rs` lines 46 to 52

```text
    8 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
    9 | pub struct Movie {
   11 | pub tracks : Vec < Track > ,
   13 | pub pssh : Vec < Pssh > ,
   15 | pub end : usize ,
   17 | pub
```


### 39 normalised tokens: `crates/cenc/src/tests/build.rs` lines 302 to 310, and tree `crates/apple-music/src/playback.rs` lines 326 to 340

```text
  302 | }
  304 | # [ derive ( Clone , Debug ) ]
  305 | pub struct TrafSpec {
  306 | pub track_id : u32 ,
  308 | pub description_index : Option < u32 > ,
  309 | pub default_duration : Option < u32 > ,
  310 | pub default_size : Option
```


### 39 normalised tokens: `crates/cenc/src/tests/build.rs` lines 376 to 385, and tree `crates/local/src/scan.rs` lines 46 to 54

```text
  376 | ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  384 | pub struct Expected {
  385 | pub offset : usize
```


### 38 normalised tokens: `crates/cenc/src/stream.rs` lines 323 to 328, and tree `crates/opium-core/src/growing.rs` lines 50 to 55

```text
  323 | }
  325 | impl Shared {
  326 | fn lock ( & self ) -> MutexGuard < '_ , State > {
  327 | self . state . lock ( ) . unwrap_or_else ( | e | e . into_inner ( ) )
  328 | }
```


### 38 normalised tokens: `crates/cenc/src/table.rs` lines 73 to 80, and tree `crates/widevine/src/boundary.rs` lines 54 to 61

```text
   73 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
   74 | pub ( crate ) struct Fragment {
   75 | pub start : usize ,
   76 | pub first_sample : usize ,
   77 | }
   80 | # [ derive ( Clone
```


### 38 normalised tokens: `crates/cenc/src/tests/build.rs` lines 181 to 190, and tree `crates/soundcloud/src/stream.rs` lines 196 to 209

```text
  181 | }
  182 | }
  184 | # [ derive ( Clone , Debug ) ]
  185 | pub struct TrackSpec {
  186 | pub id : u32 ,
  187 | pub timescale : u32 ,
  188 | pub entries : Vec < Entry > ,
  190 | pub trex : Option <
```


### 38 normalised tokens: `crates/cenc/src/tests/mod.rs` lines 21 to 26, and tree `crates/opium-ffi/src/lib.rs` lines 2059 to 2064

```text
   21 | fragment : sample . fragment ,
   22 | entry : sample . entry ,
   23 | key_id : sample . key_id ,
   24 | iv : sample . iv ,
   25 | iv_size : sample . iv_size ,
   26 | subsamples : sample . subsamples . to_vec (
```


### 37 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 188, and tree `crates/synced/src/lib.rs` lines 233 to 238

```text
  183 | . len ( ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments : self . fragments . len ( ) ,
  187 | }
  188 | }
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 216 to 221, and tree `crates/apple-music/src/lib.rs` lines 197 to 202

```text
  216 | # [ derive ( Clone , Debug , Default ) ]
  217 | pub struct MovieSpec {
  218 | pub tracks : Vec < TrackSpec > ,
  219 | pub pssh : Vec < Pssh > ,
  221 | pub extra : Vec < u8
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 224 to 234, and tree `crates/soundcloud/src/stream.rs` lines 197 to 209

```text
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec < u8 > ,
  234 | pub subsamples : Vec <
```


### 37 normalised tokens: `crates/cenc/src/tests/build.rs` lines 309 to 317, and tree `crates/app-desktop/src/app.rs` lines 551 to 559

```text
  309 | ,
  310 | pub default_size : Option < u32 > ,
  312 | pub explicit_base : bool ,
  313 | pub default_base_is_moof : bool ,
  314 | pub runs : Vec < RunSpec > ,
  315 | pub aux : Aux ,
  317 | pub sbgp : Vec <
```


### 36 normalised tokens: `crates/cenc/examples/index_cost.rs` lines 15 to 18, and tree `crates/local/src/scan.rs` lines 6 to 9

```text
   15 | Layout , System } ;
   16 | use std :: hint :: black_box ;
   17 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
   18 | use std :: time :: { Duration , Instant } ;
```


### 36 normalised tokens: `crates/cenc/src/boxes.rs` lines 188 to 199, and tree `crates/connect/src/lib.rs` lines 478 to 488

```text
  188 | body ,
  189 | }
  190 | }
  193 | pub fn at ( & self ) -> usize {
  194 | self . at
  195 | }
  198 | pub fn left ( & self ) -> usize {
  199 | self . header . end . saturating_sub (
```


### 36 normalised tokens: `crates/cenc/src/fragment.rs` lines 184 to 189, and tree `crates/player/src/engine.rs` lines 177 to 185

```text
  184 | track_id : u32 ,
  185 | base_data_offset : Option < usize > ,
  186 | description_index : Option < u32 > ,
  187 | default_duration : Option < u32 > ,
  188 | default_size : Option < u32 > ,
  189 | default_base_is_moof : bool ,
```


### 36 normalised tokens: `crates/cenc/src/movie.rs` lines 17 to 28, and tree `crates/app-desktop/src/theme.rs` lines 100 to 108

```text
   17 | at : usize ,
   18 | }
   21 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   22 | pub struct Track {
   24 | pub id : u32 ,
   26 | pub timescale : u32 ,
   28 | pub entries : Vec
```


### 36 normalised tokens: `crates/cenc/src/stream.rs` lines 312 to 318, and tree `crates/downloads/src/store.rs` lines 264 to 270

```text
  312 | duration ,
  313 | fragment : sample . fragment ,
  314 | entry : sample . entry ,
  315 | key_id : sample . key_id ,
  316 | iv : sample . iv ,
  317 | iv_size : sample . iv_size ,
  318 | subsamples : runs ,
```


### 36 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 18 to 20, and tree `crates/opium-core/src/queue.rs` lines 465 to 467

```text
   18 | let mut starts : Vec < usize > = table . iter ( ) . map ( | s | s . fragment ) . collect ( ) ;
   19 | starts . dedup ( ) ;
   20 | starts
```


### 35 normalised tokens: `crates/cenc/src/indexer.rs` lines 20 to 28, and tree `crates/connect/src/lib.rs` lines 282 to 288

```text
   20 | tracks : Vec < Track > ,
   22 | position : usize ,
   24 | scanned_to : usize ,
   25 | table : SampleTable ,
   27 | mdats : Vec < ( usize , usize ) > ,
   28 | descriptions : HashMap < Description
```


### 35 normalised tokens: `crates/cenc/src/movie.rs` lines 36 to 46, and tree `crates/app-desktop/src/theme.rs` lines 22 to 27

```text
   36 | Copy , Debug , Default , PartialEq , Eq ) ]
   37 | pub struct TrackDefaults {
   39 | pub sample_description_index : u32 ,
   41 | pub sample_duration : u32 ,
   43 | pub sample_size : u32 ,
   46 | pub sample_flags : u32 ,
```


### 35 normalised tokens: `crates/cenc/src/tests/build.rs` lines 376 to 383, and tree `crates/opium-core/src/model.rs` lines 287 to 295

```text
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 175 to 177, and tree `crates/opium-core/src/queue.rs` lines 434 to 436

```text
  175 | ) ;
  176 | let entries : Vec < u32 > = table . iter ( ) . map ( | s | s . entry ) . collect ( ) ;
  177 | assert_eq ! ( entries ,
```


### 35 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 198 to 200, and tree `crates/opium-core/src/queue.rs` lines 434 to 436

```text
  198 | ) ;
  199 | let durations : Vec < u32 > = table . iter ( ) . map ( | s | s . duration ) . collect ( ) ;
  200 | assert_eq ! ( durations ,
```


### 34 normalised tokens: `crates/cenc/src/boxes.rs` lines 15 to 23, and tree `crates/app-desktop/src/ui/menu.rs` lines 402 to 412

```text
   15 | Debug , PartialEq , Eq ) ]
   16 | pub ( crate ) struct Header {
   18 | pub at : usize ,
   19 | pub kind : Kind ,
   21 | pub body : usize ,
   23 | pub end : usize ,
```


### 34 normalised tokens: `crates/cenc/src/fragment.rs` lines 4 to 8, and tree `crates/widevine/src/host.rs` lines 25 to 28

```text
    4 | :: collections :: HashMap ;
    6 | use crate :: Error ;
    7 | use crate :: boxes :: { Body , Children , Header , Kind , skip } ;
    8 | use crate :: claims :: Claims ;
```


### 34 normalised tokens: `crates/cenc/src/stream.rs` lines 53 to 68, and tree `crates/player/src/engine.rs` lines 183 to 199

```text
   53 | ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 337 to 345, and tree `crates/soundcloud/src/types.rs` lines 102 to 109

```text
  337 | }
  339 | # [ derive ( Clone , Debug ) ]
  340 | pub struct FragmentSpec {
  341 | pub trafs : Vec < TrafSpec > ,
  343 | pub gap : Vec < u8 > ,
  345 | pub mdat64 : bool
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 375 to 383, and tree `crates/connect/src/lib.rs` lines 188 to 198

```text
  375 | ,
  376 | pub movie : MovieSpec ,
  377 | pub fragments : Vec < FragmentSpec > ,
  379 | pub trailing : Vec < u8 > ,
  380 | }
  383 | # [ derive ( Clone , Debug , PartialEq , Eq
```


### 34 normalised tokens: `crates/cenc/src/tests/build.rs` lines 814 to 821, and tree `crates/app-desktop/src/ui/grid.rs` lines 205 to 212

```text
  814 | fn traf_box (
  815 | traf : & TrafSpec ,
  816 | track : & TrackSpec ,
  817 | base : usize ,
  818 | layout : & TrafLayout ,
  819 | cleared : bool ,
  820 | ) -> Vec < u8 > {
  821 | let
```


### 34 normalised tokens: `crates/cenc/tests/allocation.rs` lines 5 to 7, and tree `crates/widevine/src/runtime.rs` lines 10 to 12

```text
    5 | std :: alloc :: { GlobalAlloc , Layout , System } ;
    6 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    7 | use std :: sync :: { Mutex , MutexGuard
```


### 33 normalised tokens: `crates/cenc/src/movie.rs` lines 21 to 30, and tree `crates/lyrics/src/lib.rs` lines 42 to 48

```text
   21 | Debug , PartialEq , Eq ) ]
   22 | pub struct Track {
   24 | pub id : u32 ,
   26 | pub timescale : u32 ,
   28 | pub entries : Vec < SampleEntry > ,
   30 | pub defaults : TrackDefaults
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 203 to 214, and tree `crates/opium-ffi/src/downloading.rs` lines 294 to 303

```text
  203 | pub fn len ( & self ) -> u64 {
  204 | self . len
  205 | }
  208 | pub fn is_empty ( & self ) -> bool {
  209 | self . len == 0
  210 | }
  214 | pub fn with_samples
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 326 to 330, and tree `crates/player/src/lib.rs` lines 251 to 255

```text
  326 | MutexGuard < '_ , State > {
  327 | self . state . lock ( ) . unwrap_or_else ( | e | e . into_inner ( ) )
  328 | }
  330 | fn decrypt_all ( & self ,
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 645 to 648, and tree `crates/apple-music/src/radio.rs` lines 847 to 850

```text
  645 | ] ;
  646 | tkhd . extend ( 1u32 . to_be_bytes ( ) ) ;
  647 | tkhd . extend ( [ 0 ; 68 ] ) ;
  648 | let mut mdhd = vec ! [ 0 ;
```


### 33 normalised tokens: `crates/cenc/src/stream.rs` lines 979 to 981, and tree `crates/downloads/src/sealed.rs` lines 285 to 287

```text
  979 | ) . len ( ) ;
  980 | let mut out = Vec :: new ( ) ;
  981 | assert ! ( stream . read_to_end ( & mut out ) . is_err ( ) ) ;
```


### 33 normalised tokens: `crates/cenc/src/table.rs` lines 34 to 43, and tree `crates/connect/src/lib.rs` lines 98 to 104

```text
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
```


### 32 normalised tokens: `crates/cenc/src/boxes.rs` lines 15 to 19, and tree `crates/widevine/src/boundary.rs` lines 54 to 57

```text
   15 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
   16 | pub ( crate ) struct Header {
   18 | pub at : usize ,
   19 | pub kind : Kind ,
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 106 to 112, and tree `crates/player/src/engine.rs` lines 177 to 185

```text
  106 | ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 107 to 114, and tree `crates/player/src/engine.rs` lines 177 to 185

```text
  107 | ,
  108 | senc : Option < Header > ,
  109 | saiz : Option < Header > ,
  110 | saio : Option < Header > ,
  112 | sbgp : Option < Header > ,
  114 | groups : Vec
```


### 32 normalised tokens: `crates/cenc/src/fragment.rs` lines 300 to 302, and tree `crates/apple-music/src/radio.rs` lines 690 to 692

```text
  300 | truns . len ( ) ) ;
  301 | for ( n , trun_box ) in boxes . truns . iter ( ) . enumerate ( ) {
  302 | let before = cx . table
```


### 32 normalised tokens: `crates/cenc/src/movie.rs` lines 114 to 117, and tree `crates/widevine/src/boundary.rs` lines 54 to 57

```text
  114 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
  115 | pub ( crate ) struct Group {
  116 | pub protected : bool ,
  117 | pub iv_size : u8 ,
```


### 32 normalised tokens: `crates/cenc/src/stream.rs` lines 893 to 896, and tree `crates/downloads/src/sealed.rs` lines 247 to 250

```text
  893 | None ) ;
  894 | let mut out = Vec :: new ( ) ;
  895 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  896 | assert_eq ! ( out , clear
```


### 32 normalised tokens: `crates/cenc/src/stream.rs` lines 911 to 914, and tree `crates/downloads/src/sealed.rs` lines 247 to 250

```text
  911 | None ) ;
  912 | let mut out = Vec :: new ( ) ;
  913 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  914 | assert_eq ! ( out , clear
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 32 to 43, and tree `crates/opium-core/src/theme.rs` lines 314 to 321

```text
   32 | ,
   34 | pub protected : bool ,
   36 | pub scheme : Scheme ,
   39 | pub pattern : Pattern ,
   40 | }
   43 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq
```


### 32 normalised tokens: `crates/cenc/src/table.rs` lines 96 to 101, and tree `crates/player/src/engine.rs` lines 177 to 185

```text
   96 | ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 250 to 257, and tree `crates/app-desktop/src/ui/tracks.rs` lines 215 to 224

```text
  250 | pub struct RunSpec {
  251 | pub samples : Vec < SampleSpec > ,
  253 | pub data_offset : bool ,
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool ,
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 251 to 262, and tree `crates/app-desktop/src/theme.rs` lines 107 to 116

```text
  251 | ,
  253 | pub data_offset : bool ,
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool ,
  257 | pub all_fields : bool ,
  258 | }
  260 | impl RunSpec {
  262 | pub fn of ( count : u32
```


### 32 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1030 to 1034, and tree `crates/opium-ffi/src/lib.rs` lines 2060 to 2064

```text
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv . clone (
```


### 32 normalised tokens: `crates/cenc/src/tests/clear.rs` lines 35 to 36, and tree `crates/app-desktop/src/app.rs` lines 4327 to 4328

```text
   35 | . iter ( )
   36 | . all ( | t | t . entries . iter ( ) . all ( | e | e . protection . is_none ( ) ) )
```


### 31 normalised tokens: `crates/cenc/src/fragment.rs` lines 8 to 10, and tree `crates/app-desktop/src/ui/dock.rs` lines 8 to 11

```text
    8 | :: claims :: Claims ;
    9 | use crate :: movie :: { Group , SampleEntry , Track , sgpd } ;
   10 | use crate :: table :: { Description , SampleTable } ;
```


### 31 normalised tokens: `crates/cenc/src/fragment.rs` lines 106 to 110, and tree `crates/player/src/engine.rs` lines 178 to 185

```text
  106 | tfhd : Option < Header > ,
  107 | truns : Vec < Header > ,
  108 | senc : Option < Header > ,
  109 | saiz : Option < Header > ,
  110 | saio : Option
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 75 to 86, and tree `crates/widevine/src/lib.rs` lines 75 to 86

```text
   75 | ,
   77 | pub pattern : Pattern ,
   78 | }
   81 | # [ derive ( Clone , Copy , Debug , Hash , PartialEq , Eq ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cens
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 94 to 103, and tree `crates/navidrome/src/types.rs` lines 32 to 38

```text
   94 | Eq ) ]
   95 | pub struct Pattern {
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 97 to 106, and tree `crates/app-desktop/src/theme.rs` lines 97 to 106

```text
   97 | pub crypt : u8 ,
   99 | pub skip : u8 ,
  100 | }
  103 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
  104 | pub struct Pssh {
  106 | pub system_id :
```


### 31 normalised tokens: `crates/cenc/src/movie.rs` lines 191 to 192, and tree `crates/app-desktop/src/theme.rs` lines 310 to 311

```text
  191 | {
  192 | if let Some ( ( _ , values ) ) = defaults . iter ( ) . find ( | ( id , _ ) | * id == track
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 57 to 69, and tree `crates/opium-core/src/queue.rs` lines 41 to 50

```text
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool ,
   66 | skippable_before : usize ,
   68 | reading : usize ,
   69 | }
```


### 31 normalised tokens: `crates/cenc/src/stream.rs` lines 743 to 746, and tree `crates/downloads/src/sealed.rs` lines 247 to 250

```text
  743 | ) ;
  744 | let mut out = Vec :: new ( ) ;
  745 | stream . read_to_end ( & mut out ) . unwrap ( ) ;
  746 | assert_eq ! ( out , clear
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 96 to 100, and tree `crates/player/src/engine.rs` lines 178 to 185

```text
   96 | offsets : Vec < usize > ,
   97 | fields : Vec < Fields > ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec
```


### 31 normalised tokens: `crates/cenc/src/table.rs` lines 117 to 123, and tree `crates/opium-ffi/src/downloading.rs` lines 303 to 309

```text
  117 | pub fn len ( & self ) -> usize {
  118 | self . offsets . len ( )
  119 | }
  122 | pub fn is_empty ( & self ) -> bool {
  123 | self . offsets
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 249 to 255, and tree `crates/player/src/lib.rs` lines 205 to 210

```text
  249 | Clone , Debug ) ]
  250 | pub struct RunSpec {
  251 | pub samples : Vec < SampleSpec > ,
  253 | pub data_offset : bool ,
  254 | pub per_sample_duration : bool ,
  255 | pub per_sample_size : bool
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 315 to 323, and tree `crates/desktop/src/devices.rs` lines 16 to 26

```text
  315 | ,
  317 | pub sbgp : Vec < ( u32 , u32 ) > ,
  319 | pub sgpd : Vec < Seig > ,
  320 | }
  322 | impl TrafSpec {
  323 | pub fn new ( track_id :
```


### 31 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1029 to 1034, and tree `crates/app-desktop/src/theme.rs` lines 60 to 65

```text
 1029 | Resolved {
 1030 | protected : seig . protected ,
 1031 | iv_size : seig . iv_size ,
 1032 | key_id : seig . key_id ,
 1033 | pattern : seig . pattern ,
 1034 | constant_iv : seig . constant_iv
```


### 31 normalised tokens: `crates/cenc/tests/allocation.rs` lines 5 to 7, and tree `crates/player/src/lib.rs` lines 16 to 18

```text
    5 | GlobalAlloc , Layout , System } ;
    6 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    7 | use std :: sync :: { Mutex , MutexGuard } ;
```


### 30 normalised tokens: `crates/cenc/src/claims.rs` lines 15 to 19, and tree `crates/widevine/src/tests/fake.rs` lines 120 to 123

```text
   15 | # [ derive ( Debug , Default ) ]
   16 | pub ( crate ) struct Claims {
   17 | in_order : Vec < ( usize , usize ) > ,
   19 | out_of_order : BTreeMap
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 6 to 9, and tree `crates/app-desktop/src/app.rs` lines 13 to 17

```text
    6 | ;
    7 | use crate :: boxes :: { Body , Children , Header , Kind , skip } ;
    8 | use crate :: claims :: Claims ;
    9 | use crate :: movie ::
```


### 30 normalised tokens: `crates/cenc/src/fragment.rs` lines 7 to 9, and tree `crates/app-desktop/src/ui/loading.rs` lines 4 to 7

```text
    7 | boxes :: { Body , Children , Header , Kind , skip } ;
    8 | use crate :: claims :: Claims ;
    9 | use crate :: movie :: { Group , SampleEntry
```


### 30 normalised tokens: `crates/cenc/src/movie.rs` lines 62 to 71, and tree `crates/app-desktop/src/theme.rs` lines 100 to 107

```text
   62 | children_at : usize ,
   63 | }
   66 | # [ derive ( Clone , Debug , PartialEq , Eq ) ]
   67 | pub struct Protection {
   69 | pub scheme : Scheme ,
   71 | pub key_id :
```


### 30 normalised tokens: `crates/cenc/src/stream.rs` lines 50 to 63, and tree `crates/app-desktop/src/screenshot.rs` lines 14 to 22

```text
   50 | ,
   53 | ready : usize ,
   55 | downloading : bool ,
   57 | wanted : Option < usize > ,
   58 | failed : Option < String > ,
   61 | undecryptable : bool ,
   63 | abandoned : bool
```


### 30 normalised tokens: `crates/cenc/src/table.rs` lines 12 to 25, and tree `crates/app-desktop/src/app.rs` lines 276 to 282

```text
   12 | > {
   14 | pub offset : usize ,
   16 | pub size : usize ,
   18 | pub duration : u32 ,
   20 | pub fragment : usize ,
   22 | pub entry : u32 ,
   25 | pub key_id :
```


### 30 normalised tokens: `crates/cenc/src/table.rs` lines 97 to 101, and tree `crates/app-desktop/src/ui/detail.rs` lines 204 to 209

```text
   97 | ,
   98 | description_of : Vec < u32 > ,
   99 | run_counts : Vec < u16 > ,
  100 | ivs : Vec < u8 > ,
  101 | runs : Vec < ( u32 , u32
```


### 30 normalised tokens: `crates/cenc/src/table.rs` lines 183 to 186, and tree `crates/app-desktop/src/app.rs` lines 6024 to 6027

```text
  183 | samples : self . len ( ) ,
  184 | ivs : self . ivs . len ( ) ,
  185 | runs : self . runs . len ( ) ,
  186 | fragments :
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 223 to 233, and tree `crates/downloads/src/queue.rs` lines 24 to 32

```text
  223 | bool ,
  224 | }
  227 | # [ derive ( Clone , Debug ) ]
  228 | pub struct SampleSpec {
  229 | pub size : u32 ,
  231 | pub duration : u32 ,
  233 | pub iv : Vec
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 257 to 263, and tree `crates/app-desktop/src/ui/tracks.rs` lines 32 to 37

```text
  257 | all_fields : bool ,
  258 | }
  260 | impl RunSpec {
  262 | pub fn of ( count : u32 , size : u32 , iv_size : usize ) -> Self {
  263 | let samples =
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 848 to 850, and tree `crates/app-desktop/src/ui/home.rs` lines 625 to 627

```text
  848 | . to_be_bytes ( ) ) ) ;
  849 | let mut run_end = base ;
  850 | for ( run , start ) in traf . runs . iter ( ) . zip (
```


### 30 normalised tokens: `crates/cenc/src/tests/build.rs` lines 1268 to 1270, and tree `crates/app-desktop/src/app.rs` lines 1105 to 1107

```text
 1268 | . into_iter ( )
 1269 | . find ( | ( found , _ ) | * found == name )
 1270 | . map ( | ( _ , spec ) | spec
```


### 30 normalised tokens: `crates/cenc/src/tests/layouts.rs` lines 49 to 52, and tree `crates/local/src/scan.rs` lines 433 to 436

```text
   49 | ) ;
   50 | assert_eq ! ( movie . tracks . len ( ) , 1 ) ;
   51 | let track = & movie . tracks [ 0 ] ;
   52 | assert_eq ! (
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 131 to 133, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  131 | check :: < crate :: Track > ( ) ;
  132 | check :: < crate :: TrackDefaults > ( ) ;
  133 | check :: < crate :: SampleEntry > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 132 to 134, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  132 | check :: < crate :: TrackDefaults > ( ) ;
  133 | check :: < crate :: SampleEntry > ( ) ;
  134 | check :: < crate :: Protection > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 133 to 135, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  133 | check :: < crate :: SampleEntry > ( ) ;
  134 | check :: < crate :: Protection > ( ) ;
  135 | check :: < crate :: Scheme > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 134 to 136, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  134 | check :: < crate :: Protection > ( ) ;
  135 | check :: < crate :: Scheme > ( ) ;
  136 | check :: < crate :: Pattern > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 135 to 137, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  135 | check :: < crate :: Scheme > ( ) ;
  136 | check :: < crate :: Pattern > ( ) ;
  137 | check :: < crate :: Pssh > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 136 to 138, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  136 | check :: < crate :: Pattern > ( ) ;
  137 | check :: < crate :: Pssh > ( ) ;
  138 | check :: < crate :: Indexer > ( ) ;
```


### 30 normalised tokens: `crates/cenc/src/tests/other.rs` lines 137 to 139, and tree `crates/widevine/src/tests/loader.rs` lines 276 to 278

```text
  137 | check :: < crate :: Pssh > ( ) ;
  138 | check :: < crate :: Indexer > ( ) ;
  139 | check :: < crate :: SampleTable > ( ) ;
```


## Names, strings and comments of the sources found in the new code


### kopuz-a58c385 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 25.
  `PartialEq` (boxes.rs, build.rs, error.rs, movie.rs, table.rs), `and_then` (boxes.rs, clear.rs, fragment.rs, lib.rs, stream.rs), `as_ref` (layouts.rs, movie_read.rs, mutation.rs, other.rs, stream.rs), `assert_eq` (allocation.rs, build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, indexer_prefixes.rs, layouts.rs, malformed.rs, mod.rs, movie_read.rs, mutation.rs, other.rs, stream.rs, stream_download.rs), `audio_config` (build.rs, clear.rs, index_cost.rs, layouts.rs, movie.rs), `base_offset` (build.rs), `body_end` (movie.rs), `checked_add` (boxes.rs, clear.rs, fragment.rs, indexer.rs, movie.rs), `copy_from_slice` (boxes.rs, build.rs, clear.rs, fragment.rs, index_cost.rs, movie.rs, stream.rs, table.rs), `extend_from_slice` (stream.rs, table.rs), `from_be_bytes` (boxes.rs, build.rs, clear.rs, indexer_prefixes.rs, other.rs, stream_download.rs), `is_empty` (build.rs, clear.rs, index_and_make_clear.rs, layouts.rs, other.rs, stream.rs, table.rs), `is_none` (boxes.rs, clear.rs, clear.rs, indexer.rs, indexer_prefixes.rs, layouts.rs, mod.rs, stream.rs), `is_some_and` (claims.rs, clear.rs, fragment.rs, indexer.rs, movie.rs), `iv_size` (allocation.rs, build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, stream.rs, table.rs), `key_id` (build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, other.rs, stream.rs, table.rs), `saturating_sub` (boxes.rs, indexer.rs, stream.rs), `to_be_bytes` (boxes.rs, build.rs, index_cost.rs, layouts.rs, stream.rs), `to_string` (malformed.rs, other.rs, stream.rs), `to_vec` (build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, mod.rs, movie.rs, other.rs, stream.rs, stream_download.rs), `track_id` (build.rs, fragment.rs, index_cost.rs, movie.rs), `try_into` (build.rs, other.rs), `unwrap_or` (build.rs, fragment.rs, indexer.rs, stream.rs), `unwrap_or_else` (allocation.rs, build.rs, clear.rs, layouts.rs, malformed.rs, stream.rs), `with_capacity` (fragment.rs, movie.rs, stream.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### kopuz-1d8b323 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 25.
  `PartialEq` (boxes.rs, build.rs, error.rs, movie.rs, table.rs), `and_then` (boxes.rs, clear.rs, fragment.rs, lib.rs, stream.rs), `as_ref` (layouts.rs, movie_read.rs, mutation.rs, other.rs, stream.rs), `assert_eq` (allocation.rs, build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, indexer_prefixes.rs, layouts.rs, malformed.rs, mod.rs, movie_read.rs, mutation.rs, other.rs, stream.rs, stream_download.rs), `audio_config` (build.rs, clear.rs, index_cost.rs, layouts.rs, movie.rs), `base_offset` (build.rs), `body_end` (movie.rs), `checked_add` (boxes.rs, clear.rs, fragment.rs, indexer.rs, movie.rs), `copy_from_slice` (boxes.rs, build.rs, clear.rs, fragment.rs, index_cost.rs, movie.rs, stream.rs, table.rs), `extend_from_slice` (stream.rs, table.rs), `from_be_bytes` (boxes.rs, build.rs, clear.rs, indexer_prefixes.rs, other.rs, stream_download.rs), `is_empty` (build.rs, clear.rs, index_and_make_clear.rs, layouts.rs, other.rs, stream.rs, table.rs), `is_none` (boxes.rs, clear.rs, clear.rs, indexer.rs, indexer_prefixes.rs, layouts.rs, mod.rs, stream.rs), `is_some_and` (claims.rs, clear.rs, fragment.rs, indexer.rs, movie.rs), `iv_size` (allocation.rs, build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, stream.rs, table.rs), `key_id` (build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, other.rs, stream.rs, table.rs), `saturating_sub` (boxes.rs, indexer.rs, stream.rs), `to_be_bytes` (boxes.rs, build.rs, index_cost.rs, layouts.rs, stream.rs), `to_string` (malformed.rs, other.rs, stream.rs), `to_vec` (build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, mod.rs, movie.rs, other.rs, stream.rs, stream_download.rs), `track_id` (build.rs, fragment.rs, index_cost.rs, movie.rs), `try_into` (build.rs, other.rs), `unwrap_or` (build.rs, fragment.rs, indexer.rs, stream.rs), `unwrap_or_else` (allocation.rs, build.rs, clear.rs, layouts.rs, malformed.rs, stream.rs), `with_capacity` (fragment.rs, movie.rs, stream.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### fmp4.rs@4522e68: `fmp4.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 31.
  `PartialEq` (boxes.rs, build.rs, error.rs, movie.rs, table.rs), `and_then` (boxes.rs, clear.rs, fragment.rs, lib.rs, stream.rs), `as_ref` (layouts.rs, movie_read.rs, mutation.rs, other.rs, stream.rs), `assert_eq` (allocation.rs, build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, indexer_prefixes.rs, layouts.rs, malformed.rs, mod.rs, movie_read.rs, mutation.rs, other.rs, stream.rs, stream_download.rs), `base_offset` (build.rs), `body_end` (movie.rs), `checked_add` (boxes.rs, clear.rs, fragment.rs, indexer.rs, movie.rs), `clear_fragment` (clear.rs), `constant_iv` (build.rs, fragment.rs, layouts.rs, movie.rs, table.rs), `copy_from_slice` (boxes.rs, build.rs, clear.rs, fragment.rs, index_cost.rs, movie.rs, stream.rs, table.rs), `extend_from_slice` (stream.rs, table.rs), `from_be_bytes` (boxes.rs, build.rs, clear.rs, indexer_prefixes.rs, other.rs, stream_download.rs), `get_mut` (boxes.rs, clear.rs), `into_iter` (allocation.rs, build.rs, malformed.rs, mutation.rs), `is_empty` (build.rs, clear.rs, index_and_make_clear.rs, layouts.rs, other.rs, stream.rs, table.rs), `iv_size` (allocation.rs, build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, stream.rs, table.rs), `key_id` (build.rs, fragment.rs, layouts.rs, mod.rs, movie.rs, other.rs, stream.rs, table.rs), `last_sample_duration` (clear.rs, index_and_make_clear.rs, lib.rs, other.rs), `map_or` (clear.rs, fragment.rs, index_cost.rs, indexer.rs, stream.rs), `moof_len` (build.rs), `ok_or_else` (boxes.rs, fragment.rs, indexer.rs, lib.rs, mod.rs, movie.rs, stream.rs), `saturating_sub` (boxes.rs, indexer.rs, stream.rs), `then_some` (clear.rs, indexer.rs), `to_be_bytes` (boxes.rs, build.rs, index_cost.rs, layouts.rs, stream.rs), `to_string` (malformed.rs, other.rs, stream.rs), `to_vec` (build.rs, clear.rs, index_and_make_clear.rs, index_cost.rs, mod.rs, movie.rs, other.rs, stream.rs, stream_download.rs), `track_id` (build.rs, fragment.rs, index_cost.rs, movie.rs), `try_from` (boxes.rs, build.rs, fragment.rs, index_cost.rs, stream.rs, table.rs), `try_into` (build.rs, other.rs), `unwrap_or` (build.rs, fragment.rs, indexer.rs, stream.rs), `with_capacity` (fragment.rs, movie.rs, stream.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 536; found: 0.
