# Token comparison

Targets:

- `clean side's lines of fuzz_fuzz_targets_common_mod.rs`
- `clean side's lines of fuzz_fuzz_targets_ids.rs`
- `clean side's lines of fuzz_fuzz_targets_licence_answer.rs`
- `clean side's lines of fuzz_fuzz_targets_media_playlist.rs`
- `clean side's lines of fuzz_fuzz_targets_next_page.rs`
- `clean side's lines of fuzz_fuzz_targets_opiums_own_readers.rs`
- `clean side's lines of fuzz_fuzz_targets_station_answer.rs`
- `clean side's lines of fuzz_fuzz_targets_web_playback_entry.rs`
- `clean side's lines of fuzz_fuzz_targets_web_player.rs`
- `clean side's lines of src_auth.rs`
- `clean side's lines of src_convert.rs`
- `clean side's lines of src_fuzzing.rs`
- `clean side's lines of src_lib.rs`
- `clean side's lines of src_mutation_tests.rs`
- `clean side's lines of src_playback.rs`
- `clean side's lines of src_station.rs`
- `clean side's lines of src_test_support.rs`

Sources:

- kopuz-a58c385: 764 files
- kopuz-1d8b323: 771 files
- removed@3d7b225: 4 files
- adapted@3d7b225: 4 files
- tree: 169 files

## Raw tokens, runs of 50 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 0 | - | 27 |
| kopuz-1d8b323 | 771 | 0 | - | 27 |
| removed@3d7b225 | 4 | 1 | 51 | 49 |
| adapted@3d7b225 | 4 | 3 | 141 | 49 |
| tree | 169 | 4 | 68 | 49 |

### 51 raw tokens: `clean side's lines of src_lib.rs` lines 112 to 116, and removed@3d7b225 `lib.rs` lines 42 to 46

```text
  112 | . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  113 | && ! name . is_empty ( )
  114 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  115 | } ) ;
  116 | if catalog ||
```


### 141 raw tokens: `clean side's lines of src_lib.rs` lines 51 to 76, and adapted@3d7b225 `lib.rs` lines 209 to 234

```text
   51 | reqwest :: Client :: builder ( )
   52 | . user_agent ( USER_AGENT )
   55 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   56 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   59 | . pool_idle_timeout ( std :: time :: Duration :: from_secs ( 300 ) )
   64 | . redirect ( reqwest :: redirect :: Policy :: custom ( | attempt | {
   65 | let apple = attempt
   66 | . url ( )
   67 | . host_str ( )
   68 | . is_some_and ( | host | host == "apple.com" || host . ends_with ( ".apple.com" ) ) ;
   69 | if ! apple {
   70 | attempt . stop ( )
   71 | } else if attempt . previous ( ) . len ( ) > 5 {
   72 | attempt . error ( "too many redirects" )
   73 | } else {
   74 | attempt . follow ( )
   75 | }
   76 | } ) )
```


### 62 raw tokens: `clean side's lines of src_lib.rs` lines 329 to 338, and adapted@3d7b225 `lib.rs` lines 414 to 423

```text
  329 | ) ,
  330 | _ if ! status . is_success ( ) => {
  331 | return Err ( Error :: Protocol ( format ! ( "{path}: HTTP {status}" ) ) ) ;
  332 | }
  333 | _ => { }
  334 | }
  335 | let body = resp
  336 | . text ( )
  337 | . await
  338 | . map_err ( | e | Error :: Network ( format ! ( "{path}: {e}" ) ) ) ? ;
```


### 51 raw tokens: `clean side's lines of src_lib.rs` lines 112 to 116, and adapted@3d7b225 `lib.rs` lines 960 to 964

```text
  112 | . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  113 | && ! name . is_empty ( )
  114 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  115 | } ) ;
  116 | if catalog ||
```


### 68 raw tokens: `clean side's lines of src_test_support.rs` lines 579 to 588, and tree `crates/cenc/src/stream.rs` lines 638 to 647

```text
  579 | ;
  582 | fn boxed ( kind : & [ u8 ; 4 ] , body : & [ u8 ] ) -> Vec < u8 > {
  583 | let size = u32 :: try_from ( 8 + body . len ( ) ) . expect ( "a small box" ) ;
  584 | [ & size . to_be_bytes ( ) [ .. ] , kind , body ] . concat ( )
  585 | }
  588 | fn
```


### 66 raw tokens: `clean side's lines of src_test_support.rs` lines 670 to 677, and tree `crates/cenc/src/stream.rs` lines 862 to 869

```text
  670 | :: make_clear ( & mut clear , & movie , & samples ) ;
  671 | for sample in samples . iter ( ) {
  672 | for at in sample . offset .. sample . offset + sample . size {
  673 | clear [ at ] = ( at % 251 ) as u8 ;
  674 | file [ at ] = clear [ at ] ^ 0xFF ;
  675 | }
  676 | }
  677 | (
```


### 65 raw tokens: `clean side's lines of src_test_support.rs` lines 413 to 422, and tree `crates/cenc/src/tests/build.rs` lines 1702 to 1710

```text
  413 | ( & mut self ) -> u64 {
  414 | self . 0 ^= self . 0 >> 12 ;
  415 | self . 0 ^= self . 0 << 25 ;
  416 | self . 0 ^= self . 0 >> 27 ;
  417 | self . 0 . wrapping_mul ( 0x2545_F491_4F6C_DD1D )
  418 | }
  421 | pub fn below ( & mut self , n : usize ) -> usize {
  422 | ( self .
```


### 52 raw tokens: `clean side's lines of src_test_support.rs` lines 588 to 594, and tree `crates/cenc/src/stream.rs` lines 647 to 652

```text
  588 | ( kind : & [ u8 ; 4 ] , flags : u32 , body : & [ u8 ] ) -> Vec < u8 > {
  589 | boxed ( kind , & [ & flags . to_be_bytes ( ) [ .. ] , body ] . concat ( ) )
  590 | }
  594 | fn
```


## Normalised tokens, runs of 30 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 63 | 49 | 29 |
| kopuz-1d8b323 | 771 | 63 | 49 | 29 |
| removed@3d7b225 | 4 | 12 | 155 | 29 |
| adapted@3d7b225 | 4 | 22 | 156 | 29 |
| tree | 169 | 44 | 131 | 29 |

### 49 normalised tokens: `clean side's lines of src_test_support.rs` lines 105 to 111, and kopuz-a58c385 `apps/kopuz/components/src/fields.rs` lines 137 to 144

```text
  105 | ) -> Self {
  106 | self . headers . push ( ( name . to_owned ( ) , value . to_owned ( ) ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body
```


### 48 normalised tokens: `clean side's lines of src_playback.rs` lines 1230 to 1233, and kopuz-a58c385 `daemon/kopuzd/tests/contract.rs` lines 2091 to 2094

```text
 1230 | ) ;
 1231 | let mut uris : Vec < & str > = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1232 | uris . dedup ( ) ;
 1233 | assert_eq ! ( uris
```


### 44 normalised tokens: `clean side's lines of src_station.rs` lines 33 to 46, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 348 to 358

```text
   33 | ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count :
```


### 41 normalised tokens: `clean side's lines of src_lib.rs` lines 459 to 464, and kopuz-a58c385 `daemon/core/src/integrations.rs` lines 7 to 12

```text
  459 | test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
  464 | use std :: sync ::
```


### 40 normalised tokens: `clean side's lines of src_lib.rs` lines 460 to 465, and kopuz-a58c385 `apps/legacy/pages/src/home_body.rs` lines 6 to 11

```text
  460 | USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
  464 | use std :: sync :: Arc ;
  465 | use std :: sync :: atomic
```


### 39 normalised tokens: `clean side's lines of src_playback.rs` lines 723 to 728, and kopuz-a58c385 `apps/common/client/src/lib.rs` lines 23 to 28

```text
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind ;
  727 | use serde_json :: json ;
  728 | use std :: io :: Read ;
```


### 38 normalised tokens: `clean side's lines of src_auth.rs` lines 133 to 138, and kopuz-a58c385 `daemon/core/src/video.rs` lines 145 to 148

```text
  133 | . send ( ) . await . map_err ( out_of_reach ) ? ;
  134 | let status = resp . status ( ) ;
  137 | if ! status . is_success ( ) {
  138 | return Err ( Error :: Protocol ( format
```


### 38 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 15, and kopuz-a58c385 `apps/legacy/kopuz/src/windows_titlebar.rs` lines 7 to 10

```text
   12 | :: playback :: fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
```


### 37 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 16, and kopuz-a58c385 `apps/kopuz/discover/src/shelves.rs` lines 7 to 10

```text
   12 | fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
```


### 37 normalised tokens: `clean side's lines of src_test_support.rs` lines 9 to 12, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 7 to 11

```text
    9 | SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread
```


### 36 normalised tokens: `clean side's lines of src_playback.rs` lines 464 to 475, and kopuz-a58c385 `daemon/server/src/ytmusic/actions.rs` lines 228 to 238

```text
  464 | }
  466 | }
  468 | # [ cfg ( test ) ]
  469 | mod reading_tests {
  470 | use super :: * ;
  471 | use serde_json :: json ;
  473 | # [ test ]
  474 | fn ids_are_of_one_kind_or_neither ( ) {
  475 | assert_eq ! ( id_kind (
```


### 36 normalised tokens: `clean side's lines of src_station.rs` lines 37 to 49, and kopuz-a58c385 `apps/kopuz/components/src/sync_status.rs` lines 24 to 33

```text
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize ) -> Self {
   47 | Self {
   48 | count ,
   49 | songs :
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 13, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 5 to 9

```text
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std ::
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 13, and kopuz-a58c385 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   10 | sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 52 to 56, and kopuz-a58c385 `daemon/player/src/systemint/windows.rs` lines 561 to 565

```text
   52 | headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) | v . as_str ( ) )
   56 | }
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 106 to 115, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 137 to 149

```text
  106 | ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body ;
  112 | self
  113 | }
  115 | pub fn chunked ( mut self
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 256 to 260, and kopuz-a58c385 `daemon/server/src/lyrics/apple_music.rs` lines 211 to 216

```text
  256 | }
  258 | fn read_request ( stream : & TcpStream ) -> Option < Request > {
  259 | let mut reader = BufReader :: new ( stream ) ;
  260 | let mut line = String :: new ( ) ;
```


### 35 normalised tokens: `clean side's lines of src_auth.rs` lines 660 to 671, and kopuz-a58c385 `apps/kopuz/store/src/nav.rs` lines 294 to 304

```text
  660 | ) ;
  661 | }
  662 | }
  665 | # [ cfg ( test ) ]
  666 | mod mutation_tests {
  667 | use super :: * ;
  668 | use crate :: mutation_tests :: damaged_texts ;
  670 | # [ test ]
  671 | fn web_player_pages_and_scripts ( ) {
```


### 35 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 19, and kopuz-a58c385 `apps/kopuz/components/src/lib.rs` lines 77 to 81

```text
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
   19 | pub
```


### 35 normalised tokens: `clean side's lines of src_lib.rs` lines 458 to 462, and kopuz-a58c385 `daemon/player/src/player.rs` lines 10 to 15

```text
  458 | ;
  459 | use crate :: test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
```


### 35 normalised tokens: `clean side's lines of src_station.rs` lines 33 to 40, and kopuz-a58c385 `daemon/core/src/integrations.rs` lines 288 to 294

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error
```


### 35 normalised tokens: `clean side's lines of src_test_support.rs` lines 150 to 153, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 188 to 191

```text
  150 | ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  152 | let stop = Arc :: new ( AtomicBool :: new ( false ) ) ;
  153 | let handler
```


### 34 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 10 to 11, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   10 | fn is_digits ( s : & str ) -> bool {
   11 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of src_auth.rs` lines 447 to 448, and kopuz-a58c385 `apps/legacy/components/src/common/controls/sort_control.rs` lines 185 to 186

```text
  447 | ) ;
  448 | let paths : Vec < String > = server . requests ( ) . into_iter ( ) . map ( | r | r . path ) . collect ( ) ;
```


### 34 normalised tokens: `clean side's lines of src_auth.rs` lines 709 to 713, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 37 to 42

```text
  709 | {
  710 | ! s . is_empty ( )
  711 | && s . bytes ( )
  712 | . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' || b == b'_' )
  713 | }
```


### 34 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 19, and kopuz-a58c385 `apps/kopuz/commands/src/ops.rs` lines 7 to 12

```text
   13 | Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
   19 | pub const MOST_CALLS : usize =
```


### 34 normalised tokens: `clean side's lines of src_mutation_tests.rs` lines 55 to 56, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   55 | fn is_digits ( s : & str ) -> bool {
   56 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of src_playback.rs` lines 132 to 134, and kopuz-a58c385 `apps/kopuz/store/src/palette.rs` lines 69 to 71

```text
  132 | body : & [ u8 ] ) -> Option < EncryptedFile > {
  133 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  134 | let first = playlist . segments
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 9 to 11, and kopuz-a58c385 `apps/kopuz/components/src/color_field.rs` lines 3 to 5

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 110 to 121, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 114 to 126

```text
  110 | ) -> Self {
  111 | self . body = body ;
  112 | self
  113 | }
  115 | pub fn chunked ( mut self ) -> Self {
  116 | self . chunked = true ;
  117 | self
  118 | }
  121 | pub fn hang_up (
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 184 to 192, and kopuz-a58c385 `apps/kopuz/store/src/downloads.rs` lines 288 to 295

```text
  184 | {
  185 | self . port
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner
```


### 33 normalised tokens: `clean side's lines of src_lib.rs` lines 837 to 838, and kopuz-a58c385 `daemon/db/tests/listing_queries.rs` lines 605 to 608

```text
  837 | ) . await . unwrap ( ) ;
  838 | let ids : Vec < & str > = tracks . iter ( ) . map ( | t | t . id . id
```


### 33 normalised tokens: `clean side's lines of src_playback.rs` lines 722 to 727, and kopuz-a58c385 `apps/kopuz/library/src/grid.rs` lines 6 to 10

```text
  722 | test_support :: {
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind ;
  727 | use serde_json ::
```


### 33 normalised tokens: `clean side's lines of src_playback.rs` lines 1009 to 1014, and kopuz-a58c385 `apps/kopuz/components/src/fields.rs` lines 37 to 42

```text
 1009 | {
 1010 | url : url . clone ( ) ,
 1011 | key_id : KEY_ID_BYTES . to_vec ( ) ,
 1012 | pssh : Vec :: new ( ) ,
 1013 | spatial : false ,
 1014 | init : None ,
```


### 33 normalised tokens: `clean side's lines of src_station.rs` lines 48 to 54, and kopuz-a58c385 `daemon/core/src/catalog.rs` lines 1155 to 1161

```text
   48 | ,
   49 | songs : Vec :: new ( ) ,
   50 | ids : HashSet :: new ( ) ,
   51 | sent : 0 ,
   52 | in_flight : 0 ,
   53 | fruitless : 0 ,
   54 | first_failure : None ,
```


### 33 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 15, and kopuz-a58c385 `daemon/core/src/downloads/rip.rs` lines 1 to 6

```text
   10 | AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 ::
```


### 32 normalised tokens: `clean side's lines of src_convert.rs` lines 123 to 126, and kopuz-a58c385 `daemon/server/src/ytmusic/actions.rs` lines 304 to 307

```text
  123 | ) ;
  124 | assert_eq ! ( a . genre , None ) ;
  125 | assert_eq ! ( a . year , None ) ;
  126 | assert_eq ! ( a . description , None ) ;
```


### 32 normalised tokens: `clean side's lines of src_lib.rs` lines 779 to 786, and kopuz-a58c385 `daemon/core/src/folder_scan.rs` lines 176 to 181

```text
  779 | }
  781 | # [ tokio :: test ]
  782 | async fn a_station_is_asked_side_by_side_within_its_bounds ( ) {
  785 | let arrived = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  786 | let server = station_server
```


### 32 normalised tokens: `clean side's lines of src_lib.rs` lines 906 to 909, and kopuz-a58c385 `daemon/db/tests/typed_queries.rs` lines 196 to 202

```text
  906 | ) ;
  907 | let error = am . station_tracks ( & station , 10 ) . await . unwrap_err ( ) ;
  908 | assert ! (
  909 | error . to_string ( ) . contains (
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 46 to 53, and kopuz-a58c385 `shared/utils/src/themes.rs` lines 30 to 35

```text
   46 | u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter (
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 139 to 143, and kopuz-a58c385 `apps/kopuz/settings_playback/src/lib.rs` lines 32 to 36

```text
  139 | ,
  140 | requests : Arc < Mutex < Vec < Request >> > ,
  141 | peak : Arc < AtomicUsize > ,
  142 | stop : Arc < AtomicBool > ,
  143 | accepting : Option < JoinHandle
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 186 to 192, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 247 to 252

```text
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( )
```


### 31 normalised tokens: `clean side's lines of src_auth.rs` lines 140 to 143, and kopuz-a58c385 `daemon/core/src/artwork.rs` lines 505 to 508

```text
  140 | ) ) ;
  141 | }
  142 | let mut body = Vec :: new ( ) ;
  143 | while let Some ( chunk ) = resp . chunk ( ) . await . map_err (
```


### 31 normalised tokens: `clean side's lines of src_lib.rs` lines 1001 to 1003, and kopuz-a58c385 `daemon/core/src/sources/removable.rs` lines 404 to 408

```text
 1001 | ) ;
 1002 | assert ! ( am . lyrics ( & upload ) . await . unwrap ( ) . is_none ( ) ) ;
 1003 | assert ! ( server . requests (
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 32 to 39, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 286 to 292

```text
   32 | struct Gathering {
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless :
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 35 to 46, and kopuz-a58c385 `apps/kopuz/components/src/lyric_line.rs` lines 51 to 60

```text
   35 | ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 184 to 192, and kopuz-a58c385 `apps/kopuz/onboarding/src/lib.rs` lines 144 to 152

```text
  184 | ,
  185 | duration : None ,
  186 | track_number : None ,
  187 | disc_number : None ,
  188 | artwork : None ,
  189 | explicit : false ,
  190 | quality : None ,
  191 | in_library : false ,
  192 | library_id :
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 8 to 10, and kopuz-a58c385 `daemon/db/tests/queries.rs` lines 6 to 8

```text
    8 | io :: { BufRead , BufReader , Read , Write } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 76 to 84, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 59 to 64

```text
   76 | ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
   84 | hold : Duration ,
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 205 to 210, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 48 to 52

```text
  205 | pub fn peak_in_flight ( & self ) -> usize {
  206 | self . peak . load ( Ordering :: SeqCst )
  207 | }
  210 | pub fn hosts ( & self ) -> Hosts {
```


### 30 normalised tokens: `clean side's lines of src_auth.rs` lines 152 to 165, and kopuz-a58c385 `shared/utils/build.rs` lines 17 to 21

```text
  152 | . into_owned ( ) )
  153 | }
  164 | fn main_script ( page : & str ) -> Option < & str > {
  165 | let bytes = page . as_bytes ( ) ;
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 194 to 196, and kopuz-a58c385 `apps/kopuz/page_home/src/ranked.rs` lines 9 to 10

```text
  194 | fn next_page_path ( next : & str , first : & str ) -> String {
  195 | let resolved = url :: Url :: parse ( API )
  196 | . ok (
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 636 to 642, and kopuz-a58c385 `daemon/core/src/network.rs` lines 281 to 287

```text
  636 | ) ;
  637 | }
  639 | # [ tokio :: test ]
  640 | async fn a_short_wait_asked_for_is_waited_and_the_request_sent_again ( ) {
  641 | let calls = AtomicUsize :: new ( 0 ) ;
  642 | let server = apple_server (
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 722 to 726, and kopuz-a58c385 `shared/proto/src/pipe.rs` lines 32 to 36

```text
  722 | :: test_support :: {
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 949 to 951, and kopuz-a58c385 `daemon/db/tests/persistence.rs` lines 761 to 764

```text
  949 | ) ;
  950 | let resp = am . start_download ( & url ) . await . unwrap ( ) ;
  951 | assert_eq ! ( resp . content_length ( ) . is_some (
```


### 30 normalised tokens: `clean side's lines of src_station.rs` lines 39 to 49, and kopuz-a58c385 `apps/kopuz/components/src/source_status.rs` lines 70 to 78

```text
   39 | ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize ) -> Self {
   47 | Self {
   48 | count ,
   49 | songs : Vec
```


### 30 normalised tokens: `clean side's lines of src_station.rs` lines 309 to 313, and kopuz-a58c385 `apps/legacy/components/src/navigation/removable_prompt.rs` lines 12 to 16

```text
  309 | let ids : Vec < String > = gathering
  310 | . into_songs ( )
  311 | . unwrap ( )
  312 | . into_iter ( )
  313 | . map ( | t | t . id
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 8 to 10, and kopuz-a58c385 `daemon/player/src/systemint/linux.rs` lines 4 to 6

```text
    8 | } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 15, and kopuz-a58c385 `apps/kopuz/link/src/lib.rs` lines 6 to 10

```text
   10 | ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 :: Engine
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 37 to 45, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 82 to 87

```text
   37 | Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec <
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 51 to 54, and kopuz-a58c385 `daemon/server/src/sources.rs` lines 61 to 62

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 76 to 82, and kopuz-a58c385 `daemon/server/src/stream/stream_buffer.rs` lines 40 to 44

```text
   76 | status : u16 ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 259 to 262, and kopuz-a58c385 `shared/utils/src/logs.rs` lines 122 to 126

```text
  259 | ;
  260 | let mut line = String :: new ( ) ;
  261 | reader . read_line ( & mut line ) . ok ( ) ? ;
  262 | let mut parts = line
```


### 49 normalised tokens: `clean side's lines of src_test_support.rs` lines 105 to 111, and kopuz-1d8b323 `apps/kopuz/components/src/fields.rs` lines 137 to 144

```text
  105 | ) -> Self {
  106 | self . headers . push ( ( name . to_owned ( ) , value . to_owned ( ) ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body
```


### 48 normalised tokens: `clean side's lines of src_playback.rs` lines 1230 to 1233, and kopuz-1d8b323 `daemon/kopuzd/tests/contract.rs` lines 2092 to 2095

```text
 1230 | ) ;
 1231 | let mut uris : Vec < & str > = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1232 | uris . dedup ( ) ;
 1233 | assert_eq ! ( uris
```


### 44 normalised tokens: `clean side's lines of src_station.rs` lines 33 to 46, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 348 to 358

```text
   33 | ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count :
```


### 41 normalised tokens: `clean side's lines of src_lib.rs` lines 459 to 464, and kopuz-1d8b323 `daemon/core/src/integrations.rs` lines 7 to 12

```text
  459 | test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
  464 | use std :: sync ::
```


### 40 normalised tokens: `clean side's lines of src_lib.rs` lines 460 to 465, and kopuz-1d8b323 `apps/legacy/pages/src/home_body.rs` lines 6 to 11

```text
  460 | USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
  464 | use std :: sync :: Arc ;
  465 | use std :: sync :: atomic
```


### 39 normalised tokens: `clean side's lines of src_playback.rs` lines 723 to 728, and kopuz-1d8b323 `apps/common/client/src/lib.rs` lines 23 to 28

```text
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind ;
  727 | use serde_json :: json ;
  728 | use std :: io :: Read ;
```


### 38 normalised tokens: `clean side's lines of src_auth.rs` lines 133 to 138, and kopuz-1d8b323 `daemon/core/src/video.rs` lines 145 to 148

```text
  133 | . send ( ) . await . map_err ( out_of_reach ) ? ;
  134 | let status = resp . status ( ) ;
  137 | if ! status . is_success ( ) {
  138 | return Err ( Error :: Protocol ( format
```


### 38 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 15, and kopuz-1d8b323 `apps/legacy/kopuz/src/windows_titlebar.rs` lines 7 to 10

```text
   12 | :: playback :: fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
```


### 37 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 16, and kopuz-1d8b323 `apps/kopuz/discover/src/shelves.rs` lines 7 to 10

```text
   12 | fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
```


### 37 normalised tokens: `clean side's lines of src_test_support.rs` lines 9 to 12, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 7 to 11

```text
    9 | SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread
```


### 36 normalised tokens: `clean side's lines of src_playback.rs` lines 464 to 475, and kopuz-1d8b323 `daemon/server/src/ytmusic/actions.rs` lines 228 to 238

```text
  464 | }
  466 | }
  468 | # [ cfg ( test ) ]
  469 | mod reading_tests {
  470 | use super :: * ;
  471 | use serde_json :: json ;
  473 | # [ test ]
  474 | fn ids_are_of_one_kind_or_neither ( ) {
  475 | assert_eq ! ( id_kind (
```


### 36 normalised tokens: `clean side's lines of src_station.rs` lines 37 to 49, and kopuz-1d8b323 `apps/kopuz/components/src/sync_status.rs` lines 24 to 33

```text
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize ) -> Self {
   47 | Self {
   48 | count ,
   49 | songs :
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 13, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 5 to 9

```text
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std ::
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 13, and kopuz-1d8b323 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   10 | sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 52 to 56, and kopuz-1d8b323 `daemon/player/src/systemint/windows.rs` lines 561 to 565

```text
   52 | headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) | v . as_str ( ) )
   56 | }
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 106 to 115, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 137 to 149

```text
  106 | ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body ;
  112 | self
  113 | }
  115 | pub fn chunked ( mut self
```


### 36 normalised tokens: `clean side's lines of src_test_support.rs` lines 256 to 260, and kopuz-1d8b323 `daemon/server/src/lyrics/apple_music.rs` lines 211 to 216

```text
  256 | }
  258 | fn read_request ( stream : & TcpStream ) -> Option < Request > {
  259 | let mut reader = BufReader :: new ( stream ) ;
  260 | let mut line = String :: new ( ) ;
```


### 35 normalised tokens: `clean side's lines of src_auth.rs` lines 660 to 671, and kopuz-1d8b323 `apps/kopuz/store/src/nav.rs` lines 294 to 304

```text
  660 | ) ;
  661 | }
  662 | }
  665 | # [ cfg ( test ) ]
  666 | mod mutation_tests {
  667 | use super :: * ;
  668 | use crate :: mutation_tests :: damaged_texts ;
  670 | # [ test ]
  671 | fn web_player_pages_and_scripts ( ) {
```


### 35 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 19, and kopuz-1d8b323 `apps/kopuz/components/src/lib.rs` lines 77 to 81

```text
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
   19 | pub
```


### 35 normalised tokens: `clean side's lines of src_lib.rs` lines 458 to 462, and kopuz-1d8b323 `daemon/player/src/player.rs` lines 10 to 15

```text
  458 | ;
  459 | use crate :: test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
```


### 35 normalised tokens: `clean side's lines of src_station.rs` lines 33 to 40, and kopuz-1d8b323 `daemon/core/src/integrations.rs` lines 288 to 294

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error
```


### 35 normalised tokens: `clean side's lines of src_test_support.rs` lines 150 to 153, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 188 to 191

```text
  150 | ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  152 | let stop = Arc :: new ( AtomicBool :: new ( false ) ) ;
  153 | let handler
```


### 34 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 10 to 11, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   10 | fn is_digits ( s : & str ) -> bool {
   11 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of src_auth.rs` lines 447 to 448, and kopuz-1d8b323 `apps/legacy/components/src/common/controls/sort_control.rs` lines 185 to 186

```text
  447 | ) ;
  448 | let paths : Vec < String > = server . requests ( ) . into_iter ( ) . map ( | r | r . path ) . collect ( ) ;
```


### 34 normalised tokens: `clean side's lines of src_auth.rs` lines 709 to 713, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 37 to 42

```text
  709 | {
  710 | ! s . is_empty ( )
  711 | && s . bytes ( )
  712 | . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' || b == b'_' )
  713 | }
```


### 34 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 19, and kopuz-1d8b323 `apps/kopuz/commands/src/ops.rs` lines 7 to 12

```text
   13 | Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
   19 | pub const MOST_CALLS : usize =
```


### 34 normalised tokens: `clean side's lines of src_mutation_tests.rs` lines 55 to 56, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   55 | fn is_digits ( s : & str ) -> bool {
   56 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of src_playback.rs` lines 132 to 134, and kopuz-1d8b323 `apps/kopuz/store/src/palette.rs` lines 69 to 71

```text
  132 | body : & [ u8 ] ) -> Option < EncryptedFile > {
  133 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  134 | let first = playlist . segments
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 9 to 11, and kopuz-1d8b323 `apps/kopuz/components/src/color_field.rs` lines 3 to 5

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 110 to 121, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 114 to 126

```text
  110 | ) -> Self {
  111 | self . body = body ;
  112 | self
  113 | }
  115 | pub fn chunked ( mut self ) -> Self {
  116 | self . chunked = true ;
  117 | self
  118 | }
  121 | pub fn hang_up (
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 184 to 192, and kopuz-1d8b323 `apps/kopuz/store/src/downloads.rs` lines 288 to 295

```text
  184 | {
  185 | self . port
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner
```


### 33 normalised tokens: `clean side's lines of src_lib.rs` lines 837 to 838, and kopuz-1d8b323 `daemon/db/tests/listing_queries.rs` lines 605 to 608

```text
  837 | ) . await . unwrap ( ) ;
  838 | let ids : Vec < & str > = tracks . iter ( ) . map ( | t | t . id . id
```


### 33 normalised tokens: `clean side's lines of src_playback.rs` lines 722 to 727, and kopuz-1d8b323 `apps/kopuz/library/src/grid.rs` lines 6 to 10

```text
  722 | test_support :: {
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind ;
  727 | use serde_json ::
```


### 33 normalised tokens: `clean side's lines of src_playback.rs` lines 1009 to 1014, and kopuz-1d8b323 `apps/kopuz/components/src/fields.rs` lines 37 to 42

```text
 1009 | {
 1010 | url : url . clone ( ) ,
 1011 | key_id : KEY_ID_BYTES . to_vec ( ) ,
 1012 | pssh : Vec :: new ( ) ,
 1013 | spatial : false ,
 1014 | init : None ,
```


### 33 normalised tokens: `clean side's lines of src_station.rs` lines 48 to 54, and kopuz-1d8b323 `daemon/core/src/catalog.rs` lines 1155 to 1161

```text
   48 | ,
   49 | songs : Vec :: new ( ) ,
   50 | ids : HashSet :: new ( ) ,
   51 | sent : 0 ,
   52 | in_flight : 0 ,
   53 | fruitless : 0 ,
   54 | first_failure : None ,
```


### 33 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 15, and kopuz-1d8b323 `daemon/core/src/downloads/rip.rs` lines 1 to 6

```text
   10 | AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 ::
```


### 32 normalised tokens: `clean side's lines of src_convert.rs` lines 123 to 126, and kopuz-1d8b323 `daemon/server/src/ytmusic/actions.rs` lines 304 to 307

```text
  123 | ) ;
  124 | assert_eq ! ( a . genre , None ) ;
  125 | assert_eq ! ( a . year , None ) ;
  126 | assert_eq ! ( a . description , None ) ;
```


### 32 normalised tokens: `clean side's lines of src_lib.rs` lines 779 to 786, and kopuz-1d8b323 `daemon/core/src/folder_scan.rs` lines 176 to 181

```text
  779 | }
  781 | # [ tokio :: test ]
  782 | async fn a_station_is_asked_side_by_side_within_its_bounds ( ) {
  785 | let arrived = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  786 | let server = station_server
```


### 32 normalised tokens: `clean side's lines of src_lib.rs` lines 906 to 909, and kopuz-1d8b323 `daemon/db/tests/typed_queries.rs` lines 196 to 202

```text
  906 | ) ;
  907 | let error = am . station_tracks ( & station , 10 ) . await . unwrap_err ( ) ;
  908 | assert ! (
  909 | error . to_string ( ) . contains (
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 46 to 53, and kopuz-1d8b323 `shared/utils/src/themes.rs` lines 30 to 35

```text
   46 | u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter (
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 139 to 143, and kopuz-1d8b323 `apps/kopuz/settings_playback/src/lib.rs` lines 32 to 36

```text
  139 | ,
  140 | requests : Arc < Mutex < Vec < Request >> > ,
  141 | peak : Arc < AtomicUsize > ,
  142 | stop : Arc < AtomicBool > ,
  143 | accepting : Option < JoinHandle
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 186 to 192, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 247 to 252

```text
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( )
```


### 31 normalised tokens: `clean side's lines of src_auth.rs` lines 140 to 143, and kopuz-1d8b323 `daemon/core/src/artwork.rs` lines 505 to 508

```text
  140 | ) ) ;
  141 | }
  142 | let mut body = Vec :: new ( ) ;
  143 | while let Some ( chunk ) = resp . chunk ( ) . await . map_err (
```


### 31 normalised tokens: `clean side's lines of src_lib.rs` lines 1001 to 1003, and kopuz-1d8b323 `daemon/core/src/sources/removable.rs` lines 404 to 408

```text
 1001 | ) ;
 1002 | assert ! ( am . lyrics ( & upload ) . await . unwrap ( ) . is_none ( ) ) ;
 1003 | assert ! ( server . requests (
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 32 to 39, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 286 to 292

```text
   32 | struct Gathering {
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless :
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 35 to 46, and kopuz-1d8b323 `apps/kopuz/components/src/lyric_line.rs` lines 51 to 60

```text
   35 | ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize
```


### 31 normalised tokens: `clean side's lines of src_station.rs` lines 184 to 192, and kopuz-1d8b323 `apps/kopuz/onboarding/src/lib.rs` lines 144 to 152

```text
  184 | ,
  185 | duration : None ,
  186 | track_number : None ,
  187 | disc_number : None ,
  188 | artwork : None ,
  189 | explicit : false ,
  190 | quality : None ,
  191 | in_library : false ,
  192 | library_id :
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 8 to 10, and kopuz-1d8b323 `daemon/db/tests/queries.rs` lines 6 to 8

```text
    8 | io :: { BufRead , BufReader , Read , Write } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 76 to 84, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 59 to 64

```text
   76 | ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
   84 | hold : Duration ,
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 205 to 210, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 48 to 52

```text
  205 | pub fn peak_in_flight ( & self ) -> usize {
  206 | self . peak . load ( Ordering :: SeqCst )
  207 | }
  210 | pub fn hosts ( & self ) -> Hosts {
```


### 30 normalised tokens: `clean side's lines of src_auth.rs` lines 152 to 165, and kopuz-1d8b323 `shared/utils/build.rs` lines 17 to 21

```text
  152 | . into_owned ( ) )
  153 | }
  164 | fn main_script ( page : & str ) -> Option < & str > {
  165 | let bytes = page . as_bytes ( ) ;
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 194 to 196, and kopuz-1d8b323 `apps/kopuz/page_home/src/ranked.rs` lines 9 to 10

```text
  194 | fn next_page_path ( next : & str , first : & str ) -> String {
  195 | let resolved = url :: Url :: parse ( API )
  196 | . ok (
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 636 to 642, and kopuz-1d8b323 `daemon/core/src/network.rs` lines 281 to 287

```text
  636 | ) ;
  637 | }
  639 | # [ tokio :: test ]
  640 | async fn a_short_wait_asked_for_is_waited_and_the_request_sent_again ( ) {
  641 | let calls = AtomicUsize :: new ( 0 ) ;
  642 | let server = apple_server (
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 722 to 726, and kopuz-1d8b323 `shared/proto/src/pipe.rs` lines 32 to 36

```text
  722 | :: test_support :: {
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 949 to 951, and kopuz-1d8b323 `daemon/db/tests/persistence.rs` lines 761 to 764

```text
  949 | ) ;
  950 | let resp = am . start_download ( & url ) . await . unwrap ( ) ;
  951 | assert_eq ! ( resp . content_length ( ) . is_some (
```


### 30 normalised tokens: `clean side's lines of src_station.rs` lines 39 to 49, and kopuz-1d8b323 `apps/kopuz/components/src/source_status.rs` lines 70 to 78

```text
   39 | ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   46 | pub fn new ( count : usize ) -> Self {
   47 | Self {
   48 | count ,
   49 | songs : Vec
```


### 30 normalised tokens: `clean side's lines of src_station.rs` lines 309 to 313, and kopuz-1d8b323 `apps/legacy/components/src/navigation/removable_prompt.rs` lines 12 to 16

```text
  309 | let ids : Vec < String > = gathering
  310 | . into_songs ( )
  311 | . unwrap ( )
  312 | . into_iter ( )
  313 | . map ( | t | t . id
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 8 to 10, and kopuz-1d8b323 `daemon/player/src/systemint/linux.rs` lines 5 to 7

```text
    8 | } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 15, and kopuz-1d8b323 `apps/kopuz/link/src/lib.rs` lines 6 to 10

```text
   10 | ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 :: Engine
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 37 to 45, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 82 to 87

```text
   37 | Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec <
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 51 to 54, and kopuz-1d8b323 `daemon/server/src/sources.rs` lines 61 to 62

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 76 to 82, and kopuz-1d8b323 `daemon/server/src/stream/stream_buffer.rs` lines 40 to 44

```text
   76 | status : u16 ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 259 to 262, and kopuz-1d8b323 `shared/utils/src/logs.rs` lines 122 to 126

```text
  259 | ;
  260 | let mut line = String :: new ( ) ;
  261 | reader . read_line ( & mut line ) . ok ( ) ? ;
  262 | let mut parts = line
```


### 155 normalised tokens: `clean side's lines of src_lib.rs` lines 98 to 120, and removed@3d7b225 `lib.rs` lines 29 to 50

```text
   98 | }
  108 | fn apple_id ( id : & str ) -> Result < & str > {
  109 | let catalog = ! id . is_empty ( ) && id . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) ;
  110 | let prefixed = id . split_once ( '.' ) . is_some_and ( | ( kind , name ) | {
  111 | ( 1 ..= 3 ) . contains ( & kind . len ( ) )
  112 | && kind . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  113 | && ! name . is_empty ( )
  114 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  115 | } ) ;
  116 | if catalog || prefixed {
  117 | Ok ( id )
  118 | } else {
  119 | Err ( Error :: Protocol ( format ! ( "not an Apple Music id: {id:?}" ) ) )
  120 | }
```


### 94 normalised tokens: `clean side's lines of src_playback.rs` lines 297 to 312, and removed@3d7b225 `playback.rs` lines 110 to 125

```text
  297 | async fn take_licence (
  298 | & self ,
  299 | session : & Session ,
  300 | challenge : & [ u8 ] ,
  301 | adam_id : & str ,
  302 | is_library : bool ,
  303 | key_uri : & str ,
  304 | key_id : & [ u8 ; 16 ] ,
  305 | ) -> Result < ( ) > {
  306 | let licence = self
  307 | . ask_licence ( challenge , adam_id , is_library , key_uri )
  308 | . await ? ;
  309 | session
  310 | . give_licence ( & licence , key_id )
  311 | . map_err ( | e | playback_error ( e . to_string ( ) ) )
  312 | }
```


### 61 normalised tokens: `clean side's lines of src_playback.rs` lines 121 to 135, and removed@3d7b225 `playback.rs` lines 62 to 69

```text
  121 | )
  122 | }
  132 | fn catalog_file ( playlist_url : & str , body : & [ u8 ] ) -> Option < EncryptedFile > {
  133 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  134 | let first = playlist . segments . first ( ) ? ;
  135 | let key = first . key . as_ref ( ) ?
```


### 57 normalised tokens: `clean side's lines of src_playback.rs` lines 262 to 271, and removed@3d7b225 `playback.rs` lines 128 to 137

```text
  262 | async fn ask_licence (
  263 | & self ,
  264 | challenge : & [ u8 ] ,
  265 | adam_id : & str ,
  266 | is_library : bool ,
  267 | key_uri : & str ,
  268 | ) -> Result < Vec < u8 >> {
  269 | let body = serde_json :: json ! ( {
  270 | "challenge" : STANDARD . encode ( challenge ) ,
  271 | "key-system" :
```


### 53 normalised tokens: `clean side's lines of src_lib.rs` lines 299 to 306, and removed@3d7b225 `lib.rs` lines 118 to 122

```text
  299 | , self . storefront ) ;
  300 | let Some ( answer ) = self . fetch :: < types :: LyricsResponse > ( & path ) . await ? else {
  301 | continue ;
  302 | } ;
  303 | let ttml = answer
  304 | . data
  305 | . iter ( )
  306 | . find_map ( | entry | entry . attributes
```


### 34 normalised tokens: `clean side's lines of src_playback.rs` lines 271 to 280, and removed@3d7b225 `playback.rs` lines 137 to 146

```text
  271 | ,
  272 | "uri" : key_uri ,
  273 | "adamId" : adam_id ,
  274 | "isLibrary" : is_library ,
  275 | "user-initiated" : true ,
  276 | } ) ;
  277 | let resp = self
  278 | . send (
  279 | self . http
  280 | . post ( format
```


### 33 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 52 to 52, and removed@3d7b225 `lib.rs` lines 41 to 42

```text
   52 | ( 1 ..= 3 ) . contains ( & p . len ( ) ) && p . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
```


### 32 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 17 to 20, and removed@3d7b225 `lib.rs` lines 42 to 45

```text
   17 | )
   18 | && ! name . is_empty ( )
   19 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
   20 | } )
```


### 32 normalised tokens: `clean side's lines of src_playback.rs` lines 64 to 78, and removed@3d7b225 `playback.rs` lines 32 to 38

```text
   64 | ,
   65 | }
   75 | fn choose_source ( entry : & serde_json :: Value ) -> Result < Source > {
   76 | let assets = entry [ "assets" ]
   77 | . as_array ( )
   78 | . filter (
```


### 32 normalised tokens: `clean side's lines of src_playback.rs` lines 78 to 81, and removed@3d7b225 `playback.rs` lines 37 to 41

```text
   78 | )
   79 | . ok_or_else ( || playback_error ( "webPlayback returned no assets" ) ) ? ;
   80 | if let [ only ] = assets . as_slice ( )
   81 | && only [ "flavor" ] . is_null ( )
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 194 to 196, and removed@3d7b225 `lib.rs` lines 13 to 14

```text
  194 | fn next_page_path ( next : & str , first : & str ) -> String {
  195 | let resolved = url :: Url :: parse ( API )
  196 | . ok (
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 27 to 29, and removed@3d7b225 `lib.rs` lines 42 to 44

```text
   27 | )
   28 | && ! name . is_empty ( )
   29 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
```


### 156 normalised tokens: `clean side's lines of src_lib.rs` lines 98 to 121, and adapted@3d7b225 `lib.rs` lines 947 to 969

```text
   98 | }
  108 | fn apple_id ( id : & str ) -> Result < & str > {
  109 | let catalog = ! id . is_empty ( ) && id . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) ;
  110 | let prefixed = id . split_once ( '.' ) . is_some_and ( | ( kind , name ) | {
  111 | ( 1 ..= 3 ) . contains ( & kind . len ( ) )
  112 | && kind . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  113 | && ! name . is_empty ( )
  114 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  115 | } ) ;
  116 | if catalog || prefixed {
  117 | Ok ( id )
  118 | } else {
  119 | Err ( Error :: Protocol ( format ! ( "not an Apple Music id: {id:?}" ) ) )
  120 | }
  121 | }
```


### 141 normalised tokens: `clean side's lines of src_lib.rs` lines 51 to 76, and adapted@3d7b225 `lib.rs` lines 209 to 234

```text
   51 | reqwest :: Client :: builder ( )
   52 | . user_agent ( USER_AGENT )
   55 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   56 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   59 | . pool_idle_timeout ( std :: time :: Duration :: from_secs ( 300 ) )
   64 | . redirect ( reqwest :: redirect :: Policy :: custom ( | attempt | {
   65 | let apple = attempt
   66 | . url ( )
   67 | . host_str ( )
   68 | . is_some_and ( | host | host == "apple.com" || host . ends_with ( ".apple.com" ) ) ;
   69 | if ! apple {
   70 | attempt . stop ( )
   71 | } else if attempt . previous ( ) . len ( ) > 5 {
   72 | attempt . error ( "too many redirects" )
   73 | } else {
   74 | attempt . follow ( )
   75 | }
   76 | } ) )
```


### 96 normalised tokens: `clean side's lines of src_playback.rs` lines 286 to 312, and adapted@3d7b225 `playback.rs` lines 762 to 782

```text
  286 | )
  287 | }
  297 | async fn take_licence (
  298 | & self ,
  299 | session : & Session ,
  300 | challenge : & [ u8 ] ,
  301 | adam_id : & str ,
  302 | is_library : bool ,
  303 | key_uri : & str ,
  304 | key_id : & [ u8 ; 16 ] ,
  305 | ) -> Result < ( ) > {
  306 | let licence = self
  307 | . ask_licence ( challenge , adam_id , is_library , key_uri )
  308 | . await ? ;
  309 | session
  310 | . give_licence ( & licence , key_id )
  311 | . map_err ( | e | playback_error ( e . to_string ( ) ) )
  312 | }
```


### 67 normalised tokens: `clean side's lines of src_lib.rs` lines 329 to 339, and adapted@3d7b225 `lib.rs` lines 414 to 424

```text
  329 | NotSignedIn ) ,
  330 | _ if ! status . is_success ( ) => {
  331 | return Err ( Error :: Protocol ( format ! ( "{path}: HTTP {status}" ) ) ) ;
  332 | }
  333 | _ => { }
  334 | }
  335 | let body = resp
  336 | . text ( )
  337 | . await
  338 | . map_err ( | e | Error :: Network ( format ! ( "{path}: {e}" ) ) ) ? ;
  339 | station :: songs_in (
```


### 60 normalised tokens: `clean side's lines of src_playback.rs` lines 122 to 135, and adapted@3d7b225 `playback.rs` lines 208 to 214

```text
  122 | }
  132 | fn catalog_file ( playlist_url : & str , body : & [ u8 ] ) -> Option < EncryptedFile > {
  133 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  134 | let first = playlist . segments . first ( ) ? ;
  135 | let key = first . key . as_ref ( ) ?
```


### 57 normalised tokens: `clean side's lines of src_playback.rs` lines 262 to 271, and adapted@3d7b225 `playback.rs` lines 785 to 794

```text
  262 | async fn ask_licence (
  263 | & self ,
  264 | challenge : & [ u8 ] ,
  265 | adam_id : & str ,
  266 | is_library : bool ,
  267 | key_uri : & str ,
  268 | ) -> Result < Vec < u8 >> {
  269 | let body = serde_json :: json ! ( {
  270 | "challenge" : STANDARD . encode ( challenge ) ,
  271 | "key-system" :
```


### 53 normalised tokens: `clean side's lines of src_lib.rs` lines 299 to 306, and adapted@3d7b225 `lib.rs` lines 1680 to 1684

```text
  299 | , self . storefront ) ;
  300 | let Some ( answer ) = self . fetch :: < types :: LyricsResponse > ( & path ) . await ? else {
  301 | continue ;
  302 | } ;
  303 | let ttml = answer
  304 | . data
  305 | . iter ( )
  306 | . find_map ( | entry | entry . attributes
```


### 37 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 15, and adapted@3d7b225 `lib.rs` lines 26 to 28

```text
   12 | playback :: fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
```


### 34 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 16, and adapted@3d7b225 `lib.rs` lines 27 to 29

```text
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
```


### 34 normalised tokens: `clean side's lines of src_playback.rs` lines 190 to 195, and adapted@3d7b225 `playback.rs` lines 146 to 151

```text
  190 | resp : reqwest :: Response ,
  191 | again : reqwest :: RequestBuilder ,
  192 | frame : std :: time :: Duration ,
  193 | decrypt : cenc :: Decrypt ,
  194 | ) -> Result < AudioStream > {
  195 | let
```


### 34 normalised tokens: `clean side's lines of src_playback.rs` lines 271 to 280, and adapted@3d7b225 `playback.rs` lines 794 to 803

```text
  271 | ,
  272 | "uri" : key_uri ,
  273 | "adamId" : adam_id ,
  274 | "isLibrary" : is_library ,
  275 | "user-initiated" : true ,
  276 | } ) ;
  277 | let resp = self
  278 | . send (
  279 | self . http
  280 | . post ( format
```


### 33 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 52 to 52, and adapted@3d7b225 `lib.rs` lines 959 to 960

```text
   52 | ( 1 ..= 3 ) . contains ( & p . len ( ) ) && p . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
```


### 32 normalised tokens: `clean side's lines of fuzz_fuzz_targets_ids.rs` lines 17 to 20, and adapted@3d7b225 `lib.rs` lines 960 to 963

```text
   17 | )
   18 | && ! name . is_empty ( )
   19 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
   20 | } )
```


### 32 normalised tokens: `clean side's lines of src_convert.rs` lines 78 to 84, and adapted@3d7b225 `convert.rs` lines 413 to 419

```text
   78 | ) ;
   79 | assert ! ( empty_catalog . quality . is_none ( ) ) ;
   80 | }
   82 | # [ test ]
   83 | fn an_album_takes_its_real_genre_its_notes_and_its_catalog_rating ( ) {
   84 | let a = album ( read ( json
```


### 32 normalised tokens: `clean side's lines of src_playback.rs` lines 64 to 78, and adapted@3d7b225 `playback.rs` lines 91 to 97

```text
   64 | ,
   65 | }
   75 | fn choose_source ( entry : & serde_json :: Value ) -> Result < Source > {
   76 | let assets = entry [ "assets" ]
   77 | . as_array ( )
   78 | . filter (
```


### 32 normalised tokens: `clean side's lines of src_playback.rs` lines 78 to 81, and adapted@3d7b225 `playback.rs` lines 96 to 100

```text
   78 | )
   79 | . ok_or_else ( || playback_error ( "webPlayback returned no assets" ) ) ? ;
   80 | if let [ only ] = assets . as_slice ( )
   81 | && only [ "flavor" ] . is_null ( )
```


### 32 normalised tokens: `clean side's lines of src_station.rs` lines 182 to 190, and adapted@3d7b225 `lib.rs` lines 1788 to 1796

```text
  182 | ,
  183 | album : None ,
  184 | album_id : None ,
  185 | duration : None ,
  186 | track_number : None ,
  187 | disc_number : None ,
  188 | artwork : None ,
  189 | explicit : false ,
  190 | quality : None
```


### 31 normalised tokens: `clean side's lines of src_lib.rs` lines 460 to 463, and adapted@3d7b225 `lib.rs` lines 27 to 30

```text
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json ::
```


### 31 normalised tokens: `clean side's lines of src_playback.rs` lines 145 to 159, and adapted@3d7b225 `playback.rs` lines 230 to 240

```text
  145 | ,
  146 | keys : vec ! [ Key {
  147 | uri : uri . clone ( ) ,
  148 | id ,
  149 | } ] ,
  150 | } )
  151 | }
  159 | fn licence_in ( answer : & serde_json
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 194 to 196, and adapted@3d7b225 `lib.rs` lines 931 to 932

```text
  194 | fn next_page_path ( next : & str , first : & str ) -> String {
  195 | let resolved = url :: Url :: parse ( API )
  196 | . ok (
```


### 30 normalised tokens: `clean side's lines of src_lib.rs` lines 459 to 461, and adapted@3d7b225 `convert.rs` lines 5 to 7

```text
  459 | :: test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 27 to 29, and adapted@3d7b225 `lib.rs` lines 960 to 962

```text
   27 | )
   28 | && ! name . is_empty ( )
   29 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
```


### 131 normalised tokens: `clean side's lines of src_test_support.rs` lines 579 to 595, and tree `crates/cenc/src/stream.rs` lines 638 to 653

```text
  579 | ;
  582 | fn boxed ( kind : & [ u8 ; 4 ] , body : & [ u8 ] ) -> Vec < u8 > {
  583 | let size = u32 :: try_from ( 8 + body . len ( ) ) . expect ( "a small box" ) ;
  584 | [ & size . to_be_bytes ( ) [ .. ] , kind , body ] . concat ( )
  585 | }
  588 | fn full_box ( kind : & [ u8 ; 4 ] , flags : u32 , body : & [ u8 ] ) -> Vec < u8 > {
  589 | boxed ( kind , & [ & flags . to_be_bytes ( ) [ .. ] , body ] . concat ( ) )
  590 | }
  594 | fn encrypted_movie ( ) -> Vec < u8 > {
  595 | let
```


### 103 normalised tokens: `clean side's lines of src_test_support.rs` lines 406 to 422, and tree `crates/cenc/src/tests/build.rs` lines 1695 to 1710

```text
  406 | struct Rng ( u64 ) ;
  408 | impl Rng {
  409 | pub fn new ( seed : u64 ) -> Self {
  410 | Self ( seed . max ( 1 ) )
  411 | }
  413 | pub fn next_u64 ( & mut self ) -> u64 {
  414 | self . 0 ^= self . 0 >> 12 ;
  415 | self . 0 ^= self . 0 << 25 ;
  416 | self . 0 ^= self . 0 >> 27 ;
  417 | self . 0 . wrapping_mul ( 0x2545_F491_4F6C_DD1D )
  418 | }
  421 | pub fn below ( & mut self , n : usize ) -> usize {
  422 | ( self . next_u64 ( ) % n
```


### 71 normalised tokens: `clean side's lines of src_test_support.rs` lines 670 to 678, and tree `crates/cenc/src/stream.rs` lines 862 to 870

```text
  670 | :: make_clear ( & mut clear , & movie , & samples ) ;
  671 | for sample in samples . iter ( ) {
  672 | for at in sample . offset .. sample . offset + sample . size {
  673 | clear [ at ] = ( at % 251 ) as u8 ;
  674 | file [ at ] = clear [ at ] ^ 0xFF ;
  675 | }
  676 | }
  677 | ( file , clear )
  678 | }
```


### 55 normalised tokens: `clean side's lines of src_station.rs` lines 182 to 199, and tree `crates/connect/src/lib.rs` lines 1433 to 1447

```text
  182 | ,
  183 | album : None ,
  184 | album_id : None ,
  185 | duration : None ,
  186 | track_number : None ,
  187 | disc_number : None ,
  188 | artwork : None ,
  189 | explicit : false ,
  190 | quality : None ,
  191 | in_library : false ,
  192 | library_id : None ,
  193 | }
  194 | }
  199 | fn simulate ( count : usize , per_call : usize ) ->
```


### 54 normalised tokens: `clean side's lines of src_station.rs` lines 182 to 199, and tree `crates/synced/src/tests.rs` lines 15 to 29

```text
  182 | . into ( ) ] ,
  183 | album : None ,
  184 | album_id : None ,
  185 | duration : None ,
  186 | track_number : None ,
  187 | disc_number : None ,
  188 | artwork : None ,
  189 | explicit : false ,
  190 | quality : None ,
  191 | in_library : false ,
  192 | library_id : None ,
  193 | }
  194 | }
  199 | fn simulate ( count : usize
```


### 47 normalised tokens: `clean side's lines of src_test_support.rs` lines 626 to 633, and tree `crates/cenc/src/stream.rs` lines 684 to 691

```text
  626 | ) ;
  627 | let mvex = boxed ( b"mvex" , & full_box ( b"trex" , 0 , & trex ) ) ;
  628 | boxed ( b"moov" , & [ trak , mvex ] . concat ( ) )
  629 | }
  633 | fn fragment ( sequence : u32 , count : u32
```


### 46 normalised tokens: `clean side's lines of src_test_support.rs` lines 644 to 649, and tree `crates/cenc/src/stream.rs` lines 701 to 706

```text
  644 | + 8 + tfhd . len ( ) + 20 + senc . len ( ) ;
  645 | let data_offset = u32 :: try_from ( moof_size + 8 ) . expect ( "a small moof" ) ;
  646 | let trun = full_box (
  647 | b"trun" ,
  648 | 0x1 ,
  649 | & [ count
```


### 44 normalised tokens: `clean side's lines of src_test_support.rs` lines 186 to 197, and tree `crates/motion/src/lib.rs` lines 56 to 65

```text
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( ) )
  193 | . clone ( )
  194 | }
  197 | pub fn requests_to ( & self
```


### 44 normalised tokens: `clean side's lines of src_test_support.rs` lines 611 to 617, and tree `crates/cenc/src/stream.rs` lines 673 to 679

```text
  611 | ) ;
  612 | let stsd = full_box (
  613 | b"stsd" ,
  614 | 0 ,
  615 | & [ & 1u32 . to_be_bytes ( ) [ .. ] , & boxed ( b"enca" , & enca ) ] . concat ( ) ,
  616 | ) ;
  617 | let mdia = boxed (
```


### 43 normalised tokens: `clean side's lines of src_lib.rs` lines 51 to 59, and tree `crates/soundcloud/src/lib.rs` lines 149 to 155

```text
   51 | reqwest :: Client :: builder ( )
   52 | . user_agent ( USER_AGENT )
   55 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   56 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   59 | . pool_idle_timeout (
```


### 43 normalised tokens: `clean side's lines of src_test_support.rs` lines 9 to 12, and tree `crates/player/src/engine.rs` lines 5 to 8

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread ::
```


### 42 normalised tokens: `clean side's lines of fuzz_fuzz_targets_station_answer.rs` lines 20 to 27, and tree `crates/cenc/fuzz/fuzz_targets/indexer_prefixes.rs` lines 8 to 13

```text
   20 | fuzz_target ! ( | data : & [ u8 ] | {
   24 | let Some ( ( & count , rest ) ) = data . split_first ( ) else {
   25 | return ;
   26 | } ;
   27 | let count = usize :: from ( count
```


### 40 normalised tokens: `clean side's lines of src_playback.rs` lines 118 to 122, and tree `crates/widevine/src/find.rs` lines 170 to 174

```text
  118 | if rest . is_empty ( ) || ! rest . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) {
  119 | return None ;
  120 | }
  121 | rest . parse ( ) . ok ( )
  122 | }
```


### 39 normalised tokens: `clean side's lines of src_test_support.rs` lines 663 to 665, and tree `crates/cenc/src/stream.rs` lines 849 to 851

```text
  663 | fn encrypted_track ( fragments : u32 , per_fragment : u32 , size : u32 ) -> ( Vec < u8 > , Vec < u8 > ) {
  664 | let mut file = encrypted_movie ( ) ;
  665 | for sequence in
```


### 37 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 12 to 15, and tree `crates/navidrome/src/lib.rs` lines 16 to 18

```text
   12 | playback :: fuzzing :: {
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
```


### 37 normalised tokens: `clean side's lines of src_station.rs` lines 33 to 40, and tree `crates/player/src/engine.rs` lines 185 to 199

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
```


### 37 normalised tokens: `clean side's lines of src_test_support.rs` lines 681 to 686, and tree `crates/cenc/src/stream.rs` lines 729 to 734

```text
  681 | Decrypt {
  682 | Box :: new ( | bytes , _ | {
  683 | bytes . iter_mut ( ) . for_each ( | b | * b ^= 0xFF ) ;
  684 | Ok ( ( ) )
  685 | } )
  686 | }
```


### 36 normalised tokens: `clean side's lines of src_lib.rs` lines 460 to 464, and tree `crates/desktop/src/mpris.rs` lines 12 to 16

```text
  460 | Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
  464 | use std :: sync ::
```


### 35 normalised tokens: `clean side's lines of src_lib.rs` lines 12 to 25, and tree `crates/app-desktop/src/ui/lyrics.rs` lines 43 to 53

```text
   12 | # [ derive ( Debug , Clone ) ]
   13 | struct Hosts {
   15 | api : String ,
   17 | web_player : String ,
   19 | play : String ,
   21 | licence : String ,
   22 | }
   24 | impl Hosts {
   25 | fn apple (
```


### 35 normalised tokens: `clean side's lines of src_lib.rs` lines 459 to 463, and tree `crates/desktop/src/backend.rs` lines 10 to 14

```text
  459 | :: test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json ::
```


### 35 normalised tokens: `clean side's lines of src_test_support.rs` lines 654 to 657, and tree `crates/cenc/src/stream.rs` lines 713 to 716

```text
  654 | ) ;
  655 | let mdat = boxed ( b"mdat" , & vec ! [ 0 ; ( count * size ) as usize ] ) ;
  656 | [ moof , mdat ] . concat ( )
  657 | }
```


### 34 normalised tokens: `clean side's lines of src_fuzzing.rs` lines 13 to 16, and tree `crates/navidrome/src/lib.rs` lines 17 to 19

```text
   13 | Asset , CatalogFile , Chosen , Entry , aac_rate , catalog_file , enhanced_file , exoplayer_playlist ,
   14 | id_kind , licence , map_range , playback_request , song_id , variant , web_playback_entry ,
   15 | widevine_playlist ,
   16 | } ;
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 10 to 15, and tree `crates/player/src/output.rs` lines 10 to 15

```text
   10 | AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 41 to 51, and tree `crates/cenc/src/tests/build.rs` lines 314 to 323

```text
   41 | ,
   43 | pub host : String ,
   45 | pub headers : Vec < ( String , String ) > ,
   46 | pub body : Vec < u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header (
```


### 34 normalised tokens: `clean side's lines of src_test_support.rs` lines 189 to 197, and tree `crates/player/src/engine.rs` lines 88 to 95

```text
  189 | {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( ) )
  193 | . clone ( )
  194 | }
  197 | pub fn requests_to ( & self , path :
```


### 33 normalised tokens: `clean side's lines of src_lib.rs` lines 458 to 461, and tree `crates/opium-core/src/provider.rs` lines 1 to 5

```text
  458 | ;
  459 | use crate :: test_support :: {
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
```


### 33 normalised tokens: `clean side's lines of src_lib.rs` lines 460 to 463, and tree `crates/opium-ffi/src/lib.rs` lines 10 to 13

```text
  460 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  461 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  462 | } ;
  463 | use serde_json :: json ;
```


### 33 normalised tokens: `clean side's lines of src_test_support.rs` lines 149 to 151, and tree `crates/player/src/lib.rs` lines 370 to 372

```text
  149 | port ( ) ;
  150 | let requests = Arc :: new ( Mutex :: new ( Vec :: new ( ) ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new
```


### 33 normalised tokens: `clean side's lines of src_test_support.rs` lines 597 to 605, and tree `crates/cenc/src/stream.rs` lines 660 to 665

```text
  597 | ) ;
  598 | let sinf = boxed (
  599 | b"sinf" ,
  600 | & [
  601 | boxed ( b"frma" , b"mp4a" ) ,
  602 | full_box (
  603 | b"schm" ,
  604 | 0 ,
  605 | & [ & b"cenc" [ .. ] , &
```


### 33 normalised tokens: `clean side's lines of src_test_support.rs` lines 669 to 671, and tree `crates/cenc/fuzz/fuzz_targets/stream_download.rs` lines 57 to 59

```text
  669 | let mut clear = file . clone ( ) ;
  670 | cenc :: make_clear ( & mut clear , & movie , & samples ) ;
  671 | for sample in samples . iter ( )
```


### 32 normalised tokens: `clean side's lines of src_lib.rs` lines 530 to 537, and tree `crates/lyrics/src/lib.rs` lines 70 to 76

```text
  530 | }
  533 | fn page_readings ( server : & TestServer ) -> usize {
  534 | server
  535 | . requests ( )
  536 | . iter ( )
  537 | . filter ( | r | r . host_name ( )
```


### 32 normalised tokens: `clean side's lines of src_playback.rs` lines 110 to 117, and tree `crates/lyrics/src/lrc.rs` lines 24 to 29

```text
  110 | url . is_empty ( ) )
  111 | }
  116 | fn ctr_kbps ( flavor : & str ) -> Option < u32 > {
  117 | let ( _ , rest ) = flavor . split_once (
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 51 to 55, and tree `crates/downloads/src/queue.rs` lines 73 to 77

```text
   51 | {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) |
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 456 to 457, and tree `crates/widevine/src/tests/find.rs` lines 337 to 338

```text
  456 | fn mutate_bytes ( rng : & mut Rng , seed : & [ u8 ] ) -> Vec < u8 > {
  457 | let mut out = seed . to_vec ( ) ;
```


### 32 normalised tokens: `clean side's lines of src_test_support.rs` lines 496 to 498, and tree `crates/cenc/src/tests/build.rs` lines 1749 to 1751

```text
  496 | ) . map ( | _ | rng . next_u64 ( ) as u8 ) . collect ( ) ;
  497 | out . splice ( i .. i , random ) ;
  498 | }
```


### 31 normalised tokens: `clean side's lines of src_playback.rs` lines 429 to 434, and tree `crates/local/src/tags.rs` lines 192 to 198

```text
  429 | ) . map_err ( | e | e . to_string ( ) )
  430 | }
  434 | pub fn enhanced_file ( playlist_url : & str , body : & str ) -> Option <
```


### 31 normalised tokens: `clean side's lines of src_playback.rs` lines 1231 to 1232, and tree `crates/app-desktop/src/ui/playlist_dialog.rs` lines 262 to 267

```text
 1231 | = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1232 | uris . dedup (
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 37 to 45, and tree `crates/app-desktop/src/ui/menu.rs` lines 402 to 412

```text
   37 | Debug , Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec
```


### 31 normalised tokens: `clean side's lines of src_test_support.rs` lines 41 to 47, and tree `crates/soundcloud/src/lib.rs` lines 69 to 76

```text
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec < ( String , String ) > ,
   46 | pub body : Vec < u8 > ,
   47 | }
```


### 30 normalised tokens: `clean side's lines of fuzz_fuzz_targets_web_playback_entry.rs` lines 15 to 17, and tree `crates/widevine/src/find.rs` lines 170 to 172

```text
   15 | if rest . is_empty ( ) || ! rest . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) {
   16 | return None ;
   17 | }
```


### 30 normalised tokens: `clean side's lines of src_playback.rs` lines 723 to 727, and tree `crates/local/src/lib.rs` lines 15 to 19

```text
  723 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  724 | xor_decrypt ,
  725 | } ;
  726 | use opium_core :: model :: ProviderKind ;
  727 | use serde_json ::
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 51 to 54, and tree `crates/app-desktop/src/app.rs` lines 2850 to 2853

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 224 to 229, and tree `crates/player/src/output.rs` lines 203 to 208

```text
  224 | ) ) ;
  225 | if let Some ( accepting ) = self . accepting . take ( ) {
  226 | let _ = accepting . join ( ) ;
  227 | }
  228 | }
  229 | }
```


### 30 normalised tokens: `clean side's lines of src_test_support.rs` lines 624 to 626, and tree `crates/cenc/src/stream.rs` lines 680 to 682

```text
  624 | ) ;
  625 | let trak = boxed ( b"trak" , & [ full_box ( b"tkhd" , 7 , & tkhd ) , mdia ] . concat ( ) ) ;
  626 | let
```


## Names, strings and comments of the sources found in the new code


### kopuz-a58c385 api.rs: `api.rs`

- Compound identifiers of 6 characters or more: 125; found in the new code: 34.
  `DeserializeOwned` (src_convert.rs), `HashSet` (src_lib.rs, src_mutation_tests.rs, src_station.rs), `USER_AGENT` (src_lib.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_u16` (src_lib.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_str` (src_station.rs), `from_value` (src_convert.rs), `futures_util` (src_lib.rs), `into_iter` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `into_owned` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_ascii_digit` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `join_all` (src_lib.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `map_or` (src_lib.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `starts_with` (fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `station_id` (fuzz_fuzz_targets_ids.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_station.rs), `strip_prefix` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs)
- String literals of three words or more: 116; found: 0.
- Comment phrases of six words: 787; found: 0.

### kopuz-a58c385 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 11; found in the new code: 4.
  `USER_AGENT` (src_lib.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 21; found: 0.
- Comment phrases of six words: 0; found: 0.

### kopuz-a58c385 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 19.
  `PartialEq` (src_playback.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_ref` (src_lib.rs, src_playback.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `checked_add` (src_auth.rs, src_playback.rs), `extend_from_slice` (src_auth.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `saturating_sub` (src_station.rs, src_test_support.rs), `to_be_bytes` (src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `try_into` (src_playback.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs), `with_capacity` (fuzz_fuzz_targets_common_mod.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### kopuz-a58c385 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 57; found in the new code: 19.
  `AppleMusic` (src_convert.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `adam_id` (src_playback.rs), `album_id` (src_convert.rs, src_station.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `artist_id` (src_convert.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `disc_number` (src_convert.rs, src_station.rs), `from_value` (src_convert.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `track_number` (src_convert.rs, src_station.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 4; found: 0.
- Comment phrases of six words: 130; found: 0.

### kopuz-a58c385 progressive.rs: `progressive.rs`

- Compound identifiers of 6 characters or more: 83; found in the new code: 17.
  `AtomicUsize` (src_lib.rs, src_test_support.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_ref` (src_lib.rs, src_playback.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `from_millis` (src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `from_secs` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_err` (src_playback.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `read_to_end` (src_playback.rs), `saturating_sub` (src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs)
- String literals of three words or more: 34; found: 0.
- Comment phrases of six words: 806; found: 0.

### kopuz-a58c385 signin.rs: `signin.rs`

- Compound identifiers of 6 characters or more: 40; found in the new code: 11.
  `as_secs` (src_test_support.rs), `from_millis` (src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `into_iter` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 11; found: 0.
- Comment phrases of six words: 39; found: 0.

### kopuz-a58c385 stream.rs: `stream.rs`

- Compound identifiers of 6 characters or more: 131; found in the new code: 42.
  `PartialEq` (src_playback.rs), `USER_AGENT` (src_lib.rs), `adam_id` (src_playback.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_bytes` (fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_i64` (src_auth.rs), `as_object` (src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_slice` (src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `asset_url` (src_playback.rs), `content_length` (src_playback.rs), `from_slice` (src_auth.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `from_str` (src_station.rs), `general_purpose` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_err` (src_playback.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `key_uri` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `m3u8_rs` (src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `media_playlist` (src_playback.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `spawn_blocking` (src_playback.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `starts_with` (fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `then_some` (src_playback.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs), `without_url` (src_auth.rs), `write_all` (src_test_support.rs)
- String literals of three words or more: 78; found: 0.
- Comment phrases of six words: 595; found: 0.

### kopuz-a58c385 types.rs: `types.rs`

- Compound identifiers of 6 characters or more: 76; found in the new code: 5.
  `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_value` (src_convert.rs), `is_library` (src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 78; found: 0.

### kopuz-a58c385 apple_music.rs: `apple_music.rs`

- Compound identifiers of 6 characters or more: 79; found in the new code: 27.
  `PartialEq` (src_playback.rs), `USER_AGENT` (src_lib.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_object` (src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_secs` (src_auth.rs, src_lib.rs, src_test_support.rs), `from_str` (src_station.rs), `from_utf8` (fuzz_fuzz_targets_ids.rs), `is_ascii_digit` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `is_whitespace` (src_auth.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `map_or` (src_lib.rs), `or_else` (src_auth.rs), `parse_ttml` (src_lib.rs), `push_str` (src_lib.rs, src_test_support.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs)
- String literals of three words or more: 18; found: 0.
- Comment phrases of six words: 78; found: 0.

### kopuz-1d8b323 api.rs: `api.rs`

- Compound identifiers of 6 characters or more: 125; found in the new code: 34.
  `DeserializeOwned` (src_convert.rs), `HashSet` (src_lib.rs, src_mutation_tests.rs, src_station.rs), `USER_AGENT` (src_lib.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_u16` (src_lib.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_str` (src_station.rs), `from_value` (src_convert.rs), `futures_util` (src_lib.rs), `into_iter` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `into_owned` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_ascii_digit` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `join_all` (src_lib.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `map_or` (src_lib.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `starts_with` (fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `station_id` (fuzz_fuzz_targets_ids.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_station.rs), `strip_prefix` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs)
- String literals of three words or more: 116; found: 0.
- Comment phrases of six words: 787; found: 0.

### kopuz-1d8b323 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 11; found in the new code: 4.
  `USER_AGENT` (src_lib.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 21; found: 0.
- Comment phrases of six words: 0; found: 0.

### kopuz-1d8b323 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 19.
  `PartialEq` (src_playback.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_ref` (src_lib.rs, src_playback.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `checked_add` (src_auth.rs, src_playback.rs), `extend_from_slice` (src_auth.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `saturating_sub` (src_station.rs, src_test_support.rs), `to_be_bytes` (src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `try_into` (src_playback.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs), `with_capacity` (fuzz_fuzz_targets_common_mod.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### kopuz-1d8b323 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 57; found in the new code: 19.
  `AppleMusic` (src_convert.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `adam_id` (src_playback.rs), `album_id` (src_convert.rs, src_station.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `artist_id` (src_convert.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `disc_number` (src_convert.rs, src_station.rs), `from_value` (src_convert.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `track_number` (src_convert.rs, src_station.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 4; found: 0.
- Comment phrases of six words: 130; found: 0.

### kopuz-1d8b323 progressive.rs: `progressive.rs`

- Compound identifiers of 6 characters or more: 83; found in the new code: 17.
  `AtomicUsize` (src_lib.rs, src_test_support.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_ref` (src_lib.rs, src_playback.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `from_millis` (src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `from_secs` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_err` (src_playback.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `read_to_end` (src_playback.rs), `saturating_sub` (src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs)
- String literals of three words or more: 34; found: 0.
- Comment phrases of six words: 806; found: 0.

### kopuz-1d8b323 signin.rs: `signin.rs`

- Compound identifiers of 6 characters or more: 40; found in the new code: 11.
  `as_secs` (src_test_support.rs), `from_millis` (src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `into_iter` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 11; found: 0.
- Comment phrases of six words: 39; found: 0.

### kopuz-1d8b323 stream.rs: `stream.rs`

- Compound identifiers of 6 characters or more: 131; found in the new code: 42.
  `PartialEq` (src_playback.rs), `USER_AGENT` (src_lib.rs), `adam_id` (src_playback.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_bytes` (fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_i64` (src_auth.rs), `as_object` (src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_slice` (src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `asset_url` (src_playback.rs), `content_length` (src_playback.rs), `from_slice` (src_auth.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `from_str` (src_station.rs), `general_purpose` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_err` (src_playback.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `key_uri` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `m3u8_rs` (src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `media_playlist` (src_playback.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `spawn_blocking` (src_playback.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `starts_with` (fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `then_some` (src_playback.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_default` (fuzz_fuzz_targets_web_playback_entry.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs), `without_url` (src_auth.rs), `write_all` (src_test_support.rs)
- String literals of three words or more: 78; found: 0.
- Comment phrases of six words: 595; found: 0.

### kopuz-1d8b323 types.rs: `types.rs`

- Compound identifiers of 6 characters or more: 76; found in the new code: 5.
  `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_value` (src_convert.rs), `is_library` (src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 78; found: 0.

### kopuz-1d8b323 apple_music.rs: `apple_music.rs`

- Compound identifiers of 6 characters or more: 79; found in the new code: 27.
  `PartialEq` (src_playback.rs), `USER_AGENT` (src_lib.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_object` (src_playback.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `catalog_id` (src_convert.rs), `from_secs` (src_auth.rs, src_lib.rs, src_test_support.rs), `from_str` (src_station.rs), `from_utf8` (fuzz_fuzz_targets_ids.rs), `is_ascii_digit` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `is_whitespace` (src_auth.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `map_or` (src_lib.rs), `or_else` (src_auth.rs), `parse_ttml` (src_lib.rs), `push_str` (src_lib.rs, src_test_support.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `to_vec` (fuzz_fuzz_targets_station_answer.rs, src_playback.rs, src_test_support.rs), `unwrap_or` (fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs)
- String literals of three words or more: 18; found: 0.
- Comment phrases of six words: 78; found: 0.

### removed@3d7b225 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 10; found in the new code: 9.
  `LazyLock` (src_auth.rs), `WEB_PLAYER` (src_auth.rs, src_lib.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `developer_token` (src_auth.rs, src_lib.rs), `is_success` (src_auth.rs, src_lib.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `opium_core` (src_auth.rs, src_playback.rs, src_station.rs), `to_owned` (src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs)
- String literals of three words or more: 3; found: 0.
- Comment phrases of six words: 39; found: 0.

### removed@3d7b225 convert.rs: `convert.rs`

- Compound identifiers of 6 characters or more: 7; found in the new code: 4.
  `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `from_secs` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_some` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs)
- String literals of three words or more: 1; found: 0.
- Comment phrases of six words: 0; found: 0.

### removed@3d7b225 lib.rs: `lib.rs`

- Compound identifiers of 6 characters or more: 34; found in the new code: 25.
  `LyricsResponse` (src_lib.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `apple_id` (fuzz_fuzz_targets_ids.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `at_offset` (fuzz_fuzz_targets_next_page.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs), `ends_with` (src_auth.rs, src_lib.rs), `find_map` (src_auth.rs, src_lib.rs), `get_or_insert` (src_station.rs), `into_iter` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs, src_test_support.rs), `into_owned` (src_auth.rs, src_lib.rs, src_test_support.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_ascii_digit` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `is_ascii_lowercase` (fuzz_fuzz_targets_ids.rs, src_lib.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_err` (src_playback.rs), `is_some_and` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs), `is_success` (src_auth.rs, src_lib.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `parse_ttml` (src_lib.rs), `query_pairs` (src_lib.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `to_owned` (src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `unwrap_or_else` (src_lib.rs, src_station.rs, src_test_support.rs), `user_token` (src_lib.rs, src_playback.rs)
- String literals of three words or more: 3; found: 2.
  - `"not an Apple Music id: {id:?}"`
  - `"the station offered no songs"`
- Comment phrases of six words: 107; found: 0.

### removed@3d7b225 playback.rs: `playback.rs`

- Compound identifiers of 6 characters or more: 49; found in the new code: 35.
  `AudioStream` (src_playback.rs), `EncryptedFile` (src_playback.rs), `PartialEq` (src_playback.rs), `adam_id` (src_playback.rs), `and_then` (fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `as_array` (src_playback.rs), `as_deref` (fuzz_fuzz_targets_web_playback_entry.rs, src_convert.rs, src_lib.rs, src_playback.rs), `as_i64` (src_auth.rs), `as_ref` (src_lib.rs, src_playback.rs), `as_slice` (src_playback.rs), `as_str` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_test_support.rs), `assert_eq` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_station_answer.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `filter_map` (fuzz_fuzz_targets_web_playback_entry.rs, src_lib.rs, src_playback.rs), `give_licence` (src_playback.rs), `is_ascii_alphanumeric` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_station.rs), `is_empty` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_licence_answer.rs, fuzz_fuzz_targets_opiums_own_readers.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `is_library` (src_playback.rs), `is_none` (fuzz_fuzz_targets_web_playback_entry.rs, src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs), `json_body` (src_playback.rs), `key_id` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `key_uri` (fuzz_fuzz_targets_media_playlist.rs, src_playback.rs), `m3u8_rs` (src_playback.rs), `map_err` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_playback.rs, src_station.rs), `map_or` (src_lib.rs), `ok_or_else` (src_auth.rs, src_playback.rs), `parse_media_playlist_res` (src_playback.rs), `playback_error` (src_playback.rs), `playlist_url` (src_playback.rs), `serde_json` (src_auth.rs, src_convert.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `split_once` (fuzz_fuzz_targets_ids.rs, fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_web_playback_entry.rs, fuzz_fuzz_targets_web_player.rs, src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `starts_with` (fuzz_fuzz_targets_media_playlist.rs, fuzz_fuzz_targets_next_page.rs, fuzz_fuzz_targets_opiums_own_readers.rs, src_auth.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs), `to_owned` (src_auth.rs, src_lib.rs, src_playback.rs, src_test_support.rs), `to_string` (src_auth.rs, src_fuzzing.rs, src_lib.rs, src_mutation_tests.rs, src_playback.rs, src_station.rs, src_test_support.rs), `try_into` (src_playback.rs), `user_token` (src_lib.rs, src_playback.rs)
- String literals of three words or more: 6; found: 4.
  - `"licence refused (errorCode {code})"`
  - `"licence response without a licence"`
  - `"no Widevine asset (only FairPlay's)"`
  - `"webPlayback returned no assets"`
- Comment phrases of six words: 72; found: 0.
