# Token comparison

Targets:

- `clean side's lines of auth.rs`
- `clean side's lines of convert.rs`
- `clean side's lines of lib.rs`
- `clean side's lines of mutation_tests.rs`
- `clean side's lines of playback.rs`
- `clean side's lines of station.rs`
- `clean side's lines of test_support.rs`

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

### 51 raw tokens: `clean side's lines of lib.rs` lines 104 to 108, and removed@3d7b225 `lib.rs` lines 42 to 46

```text
  104 | . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  105 | && ! name . is_empty ( )
  106 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  107 | } ) ;
  108 | if catalog ||
```


### 141 raw tokens: `clean side's lines of lib.rs` lines 48 to 73, and adapted@3d7b225 `lib.rs` lines 209 to 234

```text
   48 | reqwest :: Client :: builder ( )
   49 | . user_agent ( USER_AGENT )
   52 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   53 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   56 | . pool_idle_timeout ( std :: time :: Duration :: from_secs ( 300 ) )
   61 | . redirect ( reqwest :: redirect :: Policy :: custom ( | attempt | {
   62 | let apple = attempt
   63 | . url ( )
   64 | . host_str ( )
   65 | . is_some_and ( | host | host == "apple.com" || host . ends_with ( ".apple.com" ) ) ;
   66 | if ! apple {
   67 | attempt . stop ( )
   68 | } else if attempt . previous ( ) . len ( ) > 5 {
   69 | attempt . error ( "too many redirects" )
   70 | } else {
   71 | attempt . follow ( )
   72 | }
   73 | } ) )
```


### 62 raw tokens: `clean side's lines of lib.rs` lines 287 to 296, and adapted@3d7b225 `lib.rs` lines 414 to 423

```text
  287 | ) ,
  288 | _ if ! status . is_success ( ) => {
  289 | return Err ( Error :: Protocol ( format ! ( "{path}: HTTP {status}" ) ) ) ;
  290 | }
  291 | _ => { }
  292 | }
  293 | let body = resp
  294 | . text ( )
  295 | . await
  296 | . map_err ( | e | Error :: Network ( format ! ( "{path}: {e}" ) ) ) ? ;
```


### 51 raw tokens: `clean side's lines of lib.rs` lines 104 to 108, and adapted@3d7b225 `lib.rs` lines 960 to 964

```text
  104 | . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  105 | && ! name . is_empty ( )
  106 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  107 | } ) ;
  108 | if catalog ||
```


### 68 raw tokens: `clean side's lines of test_support.rs` lines 579 to 588, and tree `crates/cenc/src/stream.rs` lines 638 to 647

```text
  579 | ;
  582 | fn boxed ( kind : & [ u8 ; 4 ] , body : & [ u8 ] ) -> Vec < u8 > {
  583 | let size = u32 :: try_from ( 8 + body . len ( ) ) . expect ( "a small box" ) ;
  584 | [ & size . to_be_bytes ( ) [ .. ] , kind , body ] . concat ( )
  585 | }
  588 | fn
```


### 66 raw tokens: `clean side's lines of test_support.rs` lines 670 to 677, and tree `crates/cenc/src/stream.rs` lines 862 to 869

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


### 65 raw tokens: `clean side's lines of test_support.rs` lines 413 to 422, and tree `crates/cenc/src/tests/build.rs` lines 1702 to 1710

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


### 52 raw tokens: `clean side's lines of test_support.rs` lines 588 to 594, and tree `crates/cenc/src/stream.rs` lines 647 to 652

```text
  588 | ( kind : & [ u8 ; 4 ] , flags : u32 , body : & [ u8 ] ) -> Vec < u8 > {
  589 | boxed ( kind , & [ & flags . to_be_bytes ( ) [ .. ] , body ] . concat ( ) )
  590 | }
  594 | fn
```


## Normalised tokens, runs of 30 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 58 | 49 | 29 |
| kopuz-1d8b323 | 771 | 58 | 49 | 29 |
| removed@3d7b225 | 4 | 10 | 155 | 29 |
| adapted@3d7b225 | 4 | 18 | 156 | 29 |
| tree | 169 | 38 | 131 | 29 |

### 49 normalised tokens: `clean side's lines of test_support.rs` lines 105 to 111, and kopuz-a58c385 `apps/kopuz/components/src/fields.rs` lines 137 to 144

```text
  105 | ) -> Self {
  106 | self . headers . push ( ( name . to_owned ( ) , value . to_owned ( ) ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body
```


### 48 normalised tokens: `clean side's lines of playback.rs` lines 1062 to 1065, and kopuz-a58c385 `daemon/kopuzd/tests/contract.rs` lines 2091 to 2094

```text
 1062 | ) ;
 1063 | let mut uris : Vec < & str > = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1064 | uris . dedup ( ) ;
 1065 | assert_eq ! ( uris
```


### 44 normalised tokens: `clean side's lines of station.rs` lines 33 to 44, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 348 to 358

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
   44 | pub fn new ( count :
```


### 41 normalised tokens: `clean side's lines of lib.rs` lines 417 to 422, and kopuz-a58c385 `daemon/core/src/integrations.rs` lines 7 to 12

```text
  417 | test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
  422 | use std :: sync ::
```


### 40 normalised tokens: `clean side's lines of lib.rs` lines 418 to 423, and kopuz-a58c385 `apps/legacy/pages/src/home_body.rs` lines 6 to 11

```text
  418 | USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
  422 | use std :: sync :: Arc ;
  423 | use std :: sync :: atomic
```


### 39 normalised tokens: `clean side's lines of playback.rs` lines 560 to 565, and kopuz-a58c385 `apps/common/client/src/lib.rs` lines 23 to 28

```text
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind ;
  564 | use serde_json :: json ;
  565 | use std :: io :: Read ;
```


### 38 normalised tokens: `clean side's lines of auth.rs` lines 124 to 127, and kopuz-a58c385 `daemon/core/src/video.rs` lines 145 to 148

```text
  124 | . send ( ) . await . map_err ( out_of_reach ) ? ;
  125 | let status = resp . status ( ) ;
  126 | if ! status . is_success ( ) {
  127 | return Err ( Error :: Network ( format
```


### 37 normalised tokens: `clean side's lines of test_support.rs` lines 9 to 12, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 7 to 11

```text
    9 | SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread
```


### 36 normalised tokens: `clean side's lines of station.rs` lines 37 to 47, and kopuz-a58c385 `apps/kopuz/components/src/sync_status.rs` lines 24 to 33

```text
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize ) -> Self {
   45 | Self {
   46 | count ,
   47 | songs :
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 13, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 5 to 9

```text
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std ::
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 13, and kopuz-a58c385 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   10 | sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 52 to 56, and kopuz-a58c385 `daemon/player/src/systemint/windows.rs` lines 561 to 565

```text
   52 | headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) | v . as_str ( ) )
   56 | }
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 106 to 115, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 137 to 149

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


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 256 to 260, and kopuz-a58c385 `daemon/server/src/lyrics/apple_music.rs` lines 211 to 216

```text
  256 | }
  258 | fn read_request ( stream : & TcpStream ) -> Option < Request > {
  259 | let mut reader = BufReader :: new ( stream ) ;
  260 | let mut line = String :: new ( ) ;
```


### 35 normalised tokens: `clean side's lines of auth.rs` lines 603 to 614, and kopuz-a58c385 `apps/kopuz/store/src/nav.rs` lines 294 to 304

```text
  603 | ) ;
  604 | }
  605 | }
  608 | # [ cfg ( test ) ]
  609 | mod mutation_tests {
  610 | use super :: * ;
  611 | use crate :: mutation_tests :: damaged_texts ;
  613 | # [ test ]
  614 | fn web_player_pages_and_scripts ( ) {
```


### 35 normalised tokens: `clean side's lines of lib.rs` lines 416 to 420, and kopuz-a58c385 `daemon/player/src/player.rs` lines 10 to 15

```text
  416 | ;
  417 | use crate :: test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
```


### 35 normalised tokens: `clean side's lines of playback.rs` lines 303 to 312, and kopuz-a58c385 `daemon/server/src/ytmusic/actions.rs` lines 229 to 238

```text
  303 | }
  305 | # [ cfg ( test ) ]
  306 | mod reading_tests {
  307 | use super :: * ;
  308 | use serde_json :: json ;
  310 | # [ test ]
  311 | fn ids_are_of_one_kind_or_neither ( ) {
  312 | assert_eq ! ( id_kind (
```


### 35 normalised tokens: `clean side's lines of station.rs` lines 33 to 40, and kopuz-a58c385 `daemon/core/src/integrations.rs` lines 288 to 294

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error
```


### 35 normalised tokens: `clean side's lines of test_support.rs` lines 150 to 153, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 188 to 191

```text
  150 | ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  152 | let stop = Arc :: new ( AtomicBool :: new ( false ) ) ;
  153 | let handler
```


### 34 normalised tokens: `clean side's lines of auth.rs` lines 427 to 428, and kopuz-a58c385 `apps/legacy/components/src/common/controls/sort_control.rs` lines 185 to 186

```text
  427 | ) ;
  428 | let paths : Vec < String > = server . requests ( ) . into_iter ( ) . map ( | r | r . path ) . collect ( ) ;
```


### 34 normalised tokens: `clean side's lines of auth.rs` lines 647 to 651, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 37 to 42

```text
  647 | {
  648 | ! s . is_empty ( )
  649 | && s . bytes ( )
  650 | . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' || b == b'_' )
  651 | }
```


### 34 normalised tokens: `clean side's lines of mutation_tests.rs` lines 55 to 56, and kopuz-a58c385 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   55 | fn is_digits ( s : & str ) -> bool {
   56 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of playback.rs` lines 123 to 125, and kopuz-a58c385 `apps/kopuz/store/src/palette.rs` lines 69 to 71

```text
  123 | body : & [ u8 ] ) -> Option < EncryptedFile > {
  124 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  125 | let first = playlist . segments
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 9 to 11, and kopuz-a58c385 `apps/kopuz/components/src/color_field.rs` lines 3 to 5

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 110 to 121, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 114 to 126

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


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 184 to 192, and kopuz-a58c385 `apps/kopuz/store/src/downloads.rs` lines 288 to 295

```text
  184 | {
  185 | self . port
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner
```


### 33 normalised tokens: `clean side's lines of lib.rs` lines 726 to 727, and kopuz-a58c385 `daemon/db/tests/listing_queries.rs` lines 605 to 608

```text
  726 | ) . await . unwrap ( ) ;
  727 | let ids : Vec < & str > = tracks . iter ( ) . map ( | t | t . id . id
```


### 33 normalised tokens: `clean side's lines of playback.rs` lines 559 to 564, and kopuz-a58c385 `apps/kopuz/library/src/grid.rs` lines 6 to 10

```text
  559 | test_support :: {
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind ;
  564 | use serde_json ::
```


### 33 normalised tokens: `clean side's lines of playback.rs` lines 841 to 846, and kopuz-a58c385 `apps/kopuz/components/src/fields.rs` lines 37 to 42

```text
  841 | {
  842 | url : url . clone ( ) ,
  843 | key_id : KEY_ID_BYTES . to_vec ( ) ,
  844 | pssh : Vec :: new ( ) ,
  845 | spatial : false ,
  846 | init : None ,
```


### 33 normalised tokens: `clean side's lines of station.rs` lines 46 to 52, and kopuz-a58c385 `daemon/core/src/catalog.rs` lines 1155 to 1161

```text
   46 | ,
   47 | songs : Vec :: new ( ) ,
   48 | ids : HashSet :: new ( ) ,
   49 | sent : 0 ,
   50 | in_flight : 0 ,
   51 | fruitless : 0 ,
   52 | first_failure : None ,
```


### 33 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 15, and kopuz-a58c385 `daemon/core/src/downloads/rip.rs` lines 1 to 6

```text
   10 | AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 ::
```


### 32 normalised tokens: `clean side's lines of convert.rs` lines 123 to 126, and kopuz-a58c385 `daemon/server/src/ytmusic/actions.rs` lines 304 to 307

```text
  123 | ) ;
  124 | assert_eq ! ( a . genre , None ) ;
  125 | assert_eq ! ( a . year , None ) ;
  126 | assert_eq ! ( a . description , None ) ;
```


### 32 normalised tokens: `clean side's lines of lib.rs` lines 668 to 675, and kopuz-a58c385 `daemon/core/src/folder_scan.rs` lines 176 to 181

```text
  668 | }
  670 | # [ tokio :: test ]
  671 | async fn a_station_is_asked_side_by_side_within_its_bounds ( ) {
  674 | let arrived = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  675 | let server = station_server
```


### 32 normalised tokens: `clean side's lines of lib.rs` lines 779 to 782, and kopuz-a58c385 `daemon/db/tests/typed_queries.rs` lines 196 to 202

```text
  779 | ) ;
  780 | let error = am . station_tracks ( & station , 10 ) . await . unwrap_err ( ) ;
  781 | assert ! (
  782 | error . to_string ( ) . contains (
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 46 to 53, and kopuz-a58c385 `shared/utils/src/themes.rs` lines 30 to 35

```text
   46 | u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter (
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 139 to 143, and kopuz-a58c385 `apps/kopuz/settings_playback/src/lib.rs` lines 32 to 36

```text
  139 | ,
  140 | requests : Arc < Mutex < Vec < Request >> > ,
  141 | peak : Arc < AtomicUsize > ,
  142 | stop : Arc < AtomicBool > ,
  143 | accepting : Option < JoinHandle
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 186 to 192, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 247 to 252

```text
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( )
```


### 31 normalised tokens: `clean side's lines of auth.rs` lines 129 to 132, and kopuz-a58c385 `daemon/core/src/artwork.rs` lines 505 to 508

```text
  129 | ) ) ;
  130 | }
  131 | let mut body = Vec :: new ( ) ;
  132 | while let Some ( chunk ) = resp . chunk ( ) . await . map_err (
```


### 31 normalised tokens: `clean side's lines of lib.rs` lines 874 to 876, and kopuz-a58c385 `daemon/core/src/sources/removable.rs` lines 404 to 408

```text
  874 | ) ;
  875 | assert ! ( am . lyrics ( & upload ) . await . unwrap ( ) . is_none ( ) ) ;
  876 | assert ! ( server . requests (
```


### 31 normalised tokens: `clean side's lines of playback.rs` lines 302 to 311, and kopuz-a58c385 `daemon/server/src/ytmusic/browse/items.rs` lines 530 to 539

```text
  302 | )
  303 | }
  305 | # [ cfg ( test ) ]
  306 | mod reading_tests {
  307 | use super :: * ;
  308 | use serde_json :: json ;
  310 | # [ test ]
  311 | fn ids_are_of_one_kind_or_neither ( ) {
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 32 to 39, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 286 to 292

```text
   32 | struct Gathering {
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless :
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 35 to 44, and kopuz-a58c385 `apps/kopuz/components/src/lyric_line.rs` lines 51 to 60

```text
   35 | ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 159 to 167, and kopuz-a58c385 `apps/kopuz/onboarding/src/lib.rs` lines 144 to 152

```text
  159 | ,
  160 | duration : None ,
  161 | track_number : None ,
  162 | disc_number : None ,
  163 | artwork : None ,
  164 | explicit : false ,
  165 | quality : None ,
  166 | in_library : false ,
  167 | library_id :
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 8 to 10, and kopuz-a58c385 `daemon/db/tests/queries.rs` lines 6 to 8

```text
    8 | io :: { BufRead , BufReader , Read , Write } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 76 to 84, and kopuz-a58c385 `apps/kopuz/components/src/cards.rs` lines 59 to 64

```text
   76 | ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
   84 | hold : Duration ,
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 205 to 210, and kopuz-a58c385 `daemon/core/src/jobs.rs` lines 48 to 52

```text
  205 | pub fn peak_in_flight ( & self ) -> usize {
  206 | self . peak . load ( Ordering :: SeqCst )
  207 | }
  210 | pub fn hosts ( & self ) -> Hosts {
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 155 to 157, and kopuz-a58c385 `apps/kopuz/page_home/src/ranked.rs` lines 9 to 10

```text
  155 | fn next_page_path ( next : & str , first : & str ) -> String {
  156 | let resolved = url :: Url :: parse ( API )
  157 | . ok (
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 525 to 531, and kopuz-a58c385 `daemon/core/src/network.rs` lines 281 to 287

```text
  525 | ) ;
  526 | }
  528 | # [ tokio :: test ]
  529 | async fn a_short_wait_asked_for_is_waited_and_the_request_sent_again ( ) {
  530 | let calls = AtomicUsize :: new ( 0 ) ;
  531 | let server = apple_server (
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 559 to 563, and kopuz-a58c385 `shared/proto/src/pipe.rs` lines 32 to 36

```text
  559 | :: test_support :: {
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 781 to 783, and kopuz-a58c385 `daemon/db/tests/persistence.rs` lines 761 to 764

```text
  781 | ) ;
  782 | let resp = am . start_download ( & url ) . await . unwrap ( ) ;
  783 | assert_eq ! ( resp . content_length ( ) . is_some (
```


### 30 normalised tokens: `clean side's lines of station.rs` lines 39 to 47, and kopuz-a58c385 `apps/kopuz/components/src/source_status.rs` lines 70 to 78

```text
   39 | ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize ) -> Self {
   45 | Self {
   46 | count ,
   47 | songs : Vec
```


### 30 normalised tokens: `clean side's lines of station.rs` lines 284 to 288, and kopuz-a58c385 `apps/legacy/components/src/navigation/removable_prompt.rs` lines 12 to 16

```text
  284 | let ids : Vec < String > = gathering
  285 | . into_songs ( )
  286 | . unwrap ( )
  287 | . into_iter ( )
  288 | . map ( | t | t . id
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 8 to 10, and kopuz-a58c385 `daemon/player/src/systemint/linux.rs` lines 4 to 6

```text
    8 | } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 15, and kopuz-a58c385 `apps/kopuz/link/src/lib.rs` lines 6 to 10

```text
   10 | ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 :: Engine
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 37 to 45, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 82 to 87

```text
   37 | Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec <
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 51 to 54, and kopuz-a58c385 `daemon/server/src/sources.rs` lines 61 to 62

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 76 to 82, and kopuz-a58c385 `daemon/server/src/stream/stream_buffer.rs` lines 40 to 44

```text
   76 | status : u16 ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 259 to 262, and kopuz-a58c385 `shared/utils/src/logs.rs` lines 122 to 126

```text
  259 | ;
  260 | let mut line = String :: new ( ) ;
  261 | reader . read_line ( & mut line ) . ok ( ) ? ;
  262 | let mut parts = line
```


### 49 normalised tokens: `clean side's lines of test_support.rs` lines 105 to 111, and kopuz-1d8b323 `apps/kopuz/components/src/fields.rs` lines 137 to 144

```text
  105 | ) -> Self {
  106 | self . headers . push ( ( name . to_owned ( ) , value . to_owned ( ) ) ) ;
  107 | self
  108 | }
  110 | pub fn bytes ( mut self , body : Vec < u8 > ) -> Self {
  111 | self . body = body
```


### 48 normalised tokens: `clean side's lines of playback.rs` lines 1062 to 1065, and kopuz-1d8b323 `daemon/kopuzd/tests/contract.rs` lines 2092 to 2095

```text
 1062 | ) ;
 1063 | let mut uris : Vec < & str > = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1064 | uris . dedup ( ) ;
 1065 | assert_eq ! ( uris
```


### 44 normalised tokens: `clean side's lines of station.rs` lines 33 to 44, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 348 to 358

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
   44 | pub fn new ( count :
```


### 41 normalised tokens: `clean side's lines of lib.rs` lines 417 to 422, and kopuz-1d8b323 `daemon/core/src/integrations.rs` lines 7 to 12

```text
  417 | test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
  422 | use std :: sync ::
```


### 40 normalised tokens: `clean side's lines of lib.rs` lines 418 to 423, and kopuz-1d8b323 `apps/legacy/pages/src/home_body.rs` lines 6 to 11

```text
  418 | USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
  422 | use std :: sync :: Arc ;
  423 | use std :: sync :: atomic
```


### 39 normalised tokens: `clean side's lines of playback.rs` lines 560 to 565, and kopuz-1d8b323 `apps/common/client/src/lib.rs` lines 23 to 28

```text
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind ;
  564 | use serde_json :: json ;
  565 | use std :: io :: Read ;
```


### 38 normalised tokens: `clean side's lines of auth.rs` lines 124 to 127, and kopuz-1d8b323 `daemon/core/src/video.rs` lines 145 to 148

```text
  124 | . send ( ) . await . map_err ( out_of_reach ) ? ;
  125 | let status = resp . status ( ) ;
  126 | if ! status . is_success ( ) {
  127 | return Err ( Error :: Network ( format
```


### 37 normalised tokens: `clean side's lines of test_support.rs` lines 9 to 12, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 7 to 11

```text
    9 | SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread
```


### 36 normalised tokens: `clean side's lines of station.rs` lines 37 to 47, and kopuz-1d8b323 `apps/kopuz/components/src/sync_status.rs` lines 24 to 33

```text
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize ) -> Self {
   45 | Self {
   46 | count ,
   47 | songs :
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 13, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 5 to 9

```text
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std ::
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 13, and kopuz-1d8b323 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   10 | sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 52 to 56, and kopuz-1d8b323 `daemon/player/src/systemint/windows.rs` lines 561 to 565

```text
   52 | headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) | v . as_str ( ) )
   56 | }
```


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 106 to 115, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 137 to 149

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


### 36 normalised tokens: `clean side's lines of test_support.rs` lines 256 to 260, and kopuz-1d8b323 `daemon/server/src/lyrics/apple_music.rs` lines 211 to 216

```text
  256 | }
  258 | fn read_request ( stream : & TcpStream ) -> Option < Request > {
  259 | let mut reader = BufReader :: new ( stream ) ;
  260 | let mut line = String :: new ( ) ;
```


### 35 normalised tokens: `clean side's lines of auth.rs` lines 603 to 614, and kopuz-1d8b323 `apps/kopuz/store/src/nav.rs` lines 294 to 304

```text
  603 | ) ;
  604 | }
  605 | }
  608 | # [ cfg ( test ) ]
  609 | mod mutation_tests {
  610 | use super :: * ;
  611 | use crate :: mutation_tests :: damaged_texts ;
  613 | # [ test ]
  614 | fn web_player_pages_and_scripts ( ) {
```


### 35 normalised tokens: `clean side's lines of lib.rs` lines 416 to 420, and kopuz-1d8b323 `daemon/player/src/player.rs` lines 10 to 15

```text
  416 | ;
  417 | use crate :: test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
```


### 35 normalised tokens: `clean side's lines of playback.rs` lines 303 to 312, and kopuz-1d8b323 `daemon/server/src/ytmusic/actions.rs` lines 229 to 238

```text
  303 | }
  305 | # [ cfg ( test ) ]
  306 | mod reading_tests {
  307 | use super :: * ;
  308 | use serde_json :: json ;
  310 | # [ test ]
  311 | fn ids_are_of_one_kind_or_neither ( ) {
  312 | assert_eq ! ( id_kind (
```


### 35 normalised tokens: `clean side's lines of station.rs` lines 33 to 40, and kopuz-1d8b323 `daemon/core/src/integrations.rs` lines 288 to 294

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error
```


### 35 normalised tokens: `clean side's lines of test_support.rs` lines 150 to 153, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 188 to 191

```text
  150 | ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  152 | let stop = Arc :: new ( AtomicBool :: new ( false ) ) ;
  153 | let handler
```


### 34 normalised tokens: `clean side's lines of auth.rs` lines 427 to 428, and kopuz-1d8b323 `apps/legacy/components/src/common/controls/sort_control.rs` lines 185 to 186

```text
  427 | ) ;
  428 | let paths : Vec < String > = server . requests ( ) . into_iter ( ) . map ( | r | r . path ) . collect ( ) ;
```


### 34 normalised tokens: `clean side's lines of auth.rs` lines 647 to 651, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 37 to 42

```text
  647 | {
  648 | ! s . is_empty ( )
  649 | && s . bytes ( )
  650 | . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' || b == b'_' )
  651 | }
```


### 34 normalised tokens: `clean side's lines of mutation_tests.rs` lines 55 to 56, and kopuz-1d8b323 `daemon/server/src/youtube_download.rs` lines 37 to 41

```text
   55 | fn is_digits ( s : & str ) -> bool {
   56 | ! s . is_empty ( ) && s . bytes ( ) . all ( | b | b . is_ascii_digit ( )
```


### 34 normalised tokens: `clean side's lines of playback.rs` lines 123 to 125, and kopuz-1d8b323 `apps/kopuz/store/src/palette.rs` lines 69 to 71

```text
  123 | body : & [ u8 ] ) -> Option < EncryptedFile > {
  124 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  125 | let first = playlist . segments
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 9 to 11, and kopuz-1d8b323 `apps/kopuz/components/src/color_field.rs` lines 3 to 5

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 110 to 121, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 114 to 126

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


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 184 to 192, and kopuz-1d8b323 `apps/kopuz/store/src/downloads.rs` lines 288 to 295

```text
  184 | {
  185 | self . port
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner
```


### 33 normalised tokens: `clean side's lines of lib.rs` lines 726 to 727, and kopuz-1d8b323 `daemon/db/tests/listing_queries.rs` lines 605 to 608

```text
  726 | ) . await . unwrap ( ) ;
  727 | let ids : Vec < & str > = tracks . iter ( ) . map ( | t | t . id . id
```


### 33 normalised tokens: `clean side's lines of playback.rs` lines 559 to 564, and kopuz-1d8b323 `apps/kopuz/library/src/grid.rs` lines 6 to 10

```text
  559 | test_support :: {
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind ;
  564 | use serde_json ::
```


### 33 normalised tokens: `clean side's lines of playback.rs` lines 841 to 846, and kopuz-1d8b323 `apps/kopuz/components/src/fields.rs` lines 37 to 42

```text
  841 | {
  842 | url : url . clone ( ) ,
  843 | key_id : KEY_ID_BYTES . to_vec ( ) ,
  844 | pssh : Vec :: new ( ) ,
  845 | spatial : false ,
  846 | init : None ,
```


### 33 normalised tokens: `clean side's lines of station.rs` lines 46 to 52, and kopuz-1d8b323 `daemon/core/src/catalog.rs` lines 1155 to 1161

```text
   46 | ,
   47 | songs : Vec :: new ( ) ,
   48 | ids : HashSet :: new ( ) ,
   49 | sent : 0 ,
   50 | in_flight : 0 ,
   51 | fruitless : 0 ,
   52 | first_failure : None ,
```


### 33 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 15, and kopuz-1d8b323 `daemon/core/src/downloads/rip.rs` lines 1 to 6

```text
   10 | AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 ::
```


### 32 normalised tokens: `clean side's lines of convert.rs` lines 123 to 126, and kopuz-1d8b323 `daemon/server/src/ytmusic/actions.rs` lines 304 to 307

```text
  123 | ) ;
  124 | assert_eq ! ( a . genre , None ) ;
  125 | assert_eq ! ( a . year , None ) ;
  126 | assert_eq ! ( a . description , None ) ;
```


### 32 normalised tokens: `clean side's lines of lib.rs` lines 668 to 675, and kopuz-1d8b323 `daemon/core/src/folder_scan.rs` lines 176 to 181

```text
  668 | }
  670 | # [ tokio :: test ]
  671 | async fn a_station_is_asked_side_by_side_within_its_bounds ( ) {
  674 | let arrived = Arc :: new ( AtomicUsize :: new ( 0 ) ) ;
  675 | let server = station_server
```


### 32 normalised tokens: `clean side's lines of lib.rs` lines 779 to 782, and kopuz-1d8b323 `daemon/db/tests/typed_queries.rs` lines 196 to 202

```text
  779 | ) ;
  780 | let error = am . station_tracks ( & station , 10 ) . await . unwrap_err ( ) ;
  781 | assert ! (
  782 | error . to_string ( ) . contains (
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 46 to 53, and kopuz-1d8b323 `shared/utils/src/themes.rs` lines 30 to 35

```text
   46 | u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter (
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 139 to 143, and kopuz-1d8b323 `apps/kopuz/settings_playback/src/lib.rs` lines 32 to 36

```text
  139 | ,
  140 | requests : Arc < Mutex < Vec < Request >> > ,
  141 | peak : Arc < AtomicUsize > ,
  142 | stop : Arc < AtomicBool > ,
  143 | accepting : Option < JoinHandle
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 186 to 192, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 247 to 252

```text
  186 | }
  189 | pub fn requests ( & self ) -> Vec < Request > {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( )
```


### 31 normalised tokens: `clean side's lines of auth.rs` lines 129 to 132, and kopuz-1d8b323 `daemon/core/src/artwork.rs` lines 505 to 508

```text
  129 | ) ) ;
  130 | }
  131 | let mut body = Vec :: new ( ) ;
  132 | while let Some ( chunk ) = resp . chunk ( ) . await . map_err (
```


### 31 normalised tokens: `clean side's lines of lib.rs` lines 874 to 876, and kopuz-1d8b323 `daemon/core/src/sources/removable.rs` lines 404 to 408

```text
  874 | ) ;
  875 | assert ! ( am . lyrics ( & upload ) . await . unwrap ( ) . is_none ( ) ) ;
  876 | assert ! ( server . requests (
```


### 31 normalised tokens: `clean side's lines of playback.rs` lines 302 to 311, and kopuz-1d8b323 `daemon/server/src/ytmusic/browse/items.rs` lines 530 to 539

```text
  302 | )
  303 | }
  305 | # [ cfg ( test ) ]
  306 | mod reading_tests {
  307 | use super :: * ;
  308 | use serde_json :: json ;
  310 | # [ test ]
  311 | fn ids_are_of_one_kind_or_neither ( ) {
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 32 to 39, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 286 to 292

```text
   32 | struct Gathering {
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless :
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 35 to 44, and kopuz-1d8b323 `apps/kopuz/components/src/lyric_line.rs` lines 51 to 60

```text
   35 | ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize
```


### 31 normalised tokens: `clean side's lines of station.rs` lines 159 to 167, and kopuz-1d8b323 `apps/kopuz/onboarding/src/lib.rs` lines 144 to 152

```text
  159 | ,
  160 | duration : None ,
  161 | track_number : None ,
  162 | disc_number : None ,
  163 | artwork : None ,
  164 | explicit : false ,
  165 | quality : None ,
  166 | in_library : false ,
  167 | library_id :
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 8 to 10, and kopuz-1d8b323 `daemon/db/tests/queries.rs` lines 6 to 8

```text
    8 | io :: { BufRead , BufReader , Read , Write } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 76 to 84, and kopuz-1d8b323 `apps/kopuz/components/src/cards.rs` lines 59 to 64

```text
   76 | ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
   84 | hold : Duration ,
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 205 to 210, and kopuz-1d8b323 `daemon/core/src/jobs.rs` lines 48 to 52

```text
  205 | pub fn peak_in_flight ( & self ) -> usize {
  206 | self . peak . load ( Ordering :: SeqCst )
  207 | }
  210 | pub fn hosts ( & self ) -> Hosts {
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 155 to 157, and kopuz-1d8b323 `apps/kopuz/page_home/src/ranked.rs` lines 9 to 10

```text
  155 | fn next_page_path ( next : & str , first : & str ) -> String {
  156 | let resolved = url :: Url :: parse ( API )
  157 | . ok (
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 525 to 531, and kopuz-1d8b323 `daemon/core/src/network.rs` lines 281 to 287

```text
  525 | ) ;
  526 | }
  528 | # [ tokio :: test ]
  529 | async fn a_short_wait_asked_for_is_waited_and_the_request_sent_again ( ) {
  530 | let calls = AtomicUsize :: new ( 0 ) ;
  531 | let server = apple_server (
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 559 to 563, and kopuz-1d8b323 `shared/proto/src/pipe.rs` lines 32 to 36

```text
  559 | :: test_support :: {
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 781 to 783, and kopuz-1d8b323 `daemon/db/tests/persistence.rs` lines 761 to 764

```text
  781 | ) ;
  782 | let resp = am . start_download ( & url ) . await . unwrap ( ) ;
  783 | assert_eq ! ( resp . content_length ( ) . is_some (
```


### 30 normalised tokens: `clean side's lines of station.rs` lines 39 to 47, and kopuz-1d8b323 `apps/kopuz/components/src/source_status.rs` lines 70 to 78

```text
   39 | ,
   40 | first_failure : Option < Error > ,
   41 | }
   43 | impl Gathering {
   44 | pub fn new ( count : usize ) -> Self {
   45 | Self {
   46 | count ,
   47 | songs : Vec
```


### 30 normalised tokens: `clean side's lines of station.rs` lines 284 to 288, and kopuz-1d8b323 `apps/legacy/components/src/navigation/removable_prompt.rs` lines 12 to 16

```text
  284 | let ids : Vec < String > = gathering
  285 | . into_songs ( )
  286 | . unwrap ( )
  287 | . into_iter ( )
  288 | . map ( | t | t . id
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 8 to 10, and kopuz-1d8b323 `daemon/player/src/systemint/linux.rs` lines 5 to 7

```text
    8 | } ;
    9 | use std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 15, and kopuz-1d8b323 `apps/kopuz/link/src/lib.rs` lines 6 to 10

```text
   10 | ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64 :: Engine
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 37 to 45, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 82 to 87

```text
   37 | Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec <
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 51 to 54, and kopuz-1d8b323 `daemon/server/src/sources.rs` lines 61 to 62

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 76 to 82, and kopuz-1d8b323 `daemon/server/src/stream/stream_buffer.rs` lines 40 to 44

```text
   76 | status : u16 ,
   77 | headers : Vec < ( String , String ) > ,
   78 | body : Vec < u8 > ,
   80 | chunked : bool ,
   82 | hang_up : bool ,
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 259 to 262, and kopuz-1d8b323 `shared/utils/src/logs.rs` lines 122 to 126

```text
  259 | ;
  260 | let mut line = String :: new ( ) ;
  261 | reader . read_line ( & mut line ) . ok ( ) ? ;
  262 | let mut parts = line
```


### 155 normalised tokens: `clean side's lines of lib.rs` lines 90 to 112, and removed@3d7b225 `lib.rs` lines 29 to 50

```text
   90 | }
  100 | fn apple_id ( id : & str ) -> Result < & str > {
  101 | let catalog = ! id . is_empty ( ) && id . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) ;
  102 | let prefixed = id . split_once ( '.' ) . is_some_and ( | ( kind , name ) | {
  103 | ( 1 ..= 3 ) . contains ( & kind . len ( ) )
  104 | && kind . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  105 | && ! name . is_empty ( )
  106 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  107 | } ) ;
  108 | if catalog || prefixed {
  109 | Ok ( id )
  110 | } else {
  111 | Err ( Error :: Protocol ( format ! ( "not an Apple Music id: {id:?}" ) ) )
  112 | }
```


### 94 normalised tokens: `clean side's lines of playback.rs` lines 276 to 291, and removed@3d7b225 `playback.rs` lines 110 to 125

```text
  276 | async fn take_licence (
  277 | & self ,
  278 | session : & Session ,
  279 | challenge : & [ u8 ] ,
  280 | adam_id : & str ,
  281 | is_library : bool ,
  282 | key_uri : & str ,
  283 | key_id : & [ u8 ; 16 ] ,
  284 | ) -> Result < ( ) > {
  285 | let licence = self
  286 | . ask_licence ( challenge , adam_id , is_library , key_uri )
  287 | . await ? ;
  288 | session
  289 | . give_licence ( & licence , key_id )
  290 | . map_err ( | e | playback_error ( e . to_string ( ) ) )
  291 | }
```


### 61 normalised tokens: `clean side's lines of playback.rs` lines 112 to 126, and removed@3d7b225 `playback.rs` lines 62 to 69

```text
  112 | )
  113 | }
  123 | fn catalog_file ( playlist_url : & str , body : & [ u8 ] ) -> Option < EncryptedFile > {
  124 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  125 | let first = playlist . segments . first ( ) ? ;
  126 | let key = first . key . as_ref ( ) ?
```


### 57 normalised tokens: `clean side's lines of playback.rs` lines 241 to 250, and removed@3d7b225 `playback.rs` lines 128 to 137

```text
  241 | async fn ask_licence (
  242 | & self ,
  243 | challenge : & [ u8 ] ,
  244 | adam_id : & str ,
  245 | is_library : bool ,
  246 | key_uri : & str ,
  247 | ) -> Result < Vec < u8 >> {
  248 | let body = serde_json :: json ! ( {
  249 | "challenge" : STANDARD . encode ( challenge ) ,
  250 | "key-system" :
```


### 53 normalised tokens: `clean side's lines of lib.rs` lines 257 to 264, and removed@3d7b225 `lib.rs` lines 118 to 122

```text
  257 | , self . storefront ) ;
  258 | let Some ( answer ) = self . fetch :: < types :: LyricsResponse > ( & path ) . await ? else {
  259 | continue ;
  260 | } ;
  261 | let ttml = answer
  262 | . data
  263 | . iter ( )
  264 | . find_map ( | entry | entry . attributes
```


### 34 normalised tokens: `clean side's lines of playback.rs` lines 250 to 259, and removed@3d7b225 `playback.rs` lines 137 to 146

```text
  250 | ,
  251 | "uri" : key_uri ,
  252 | "adamId" : adam_id ,
  253 | "isLibrary" : is_library ,
  254 | "user-initiated" : true ,
  255 | } ) ;
  256 | let resp = self
  257 | . send (
  258 | self . http
  259 | . post ( format
```


### 32 normalised tokens: `clean side's lines of playback.rs` lines 55 to 69, and removed@3d7b225 `playback.rs` lines 32 to 38

```text
   55 | ,
   56 | }
   66 | fn choose_source ( entry : & serde_json :: Value ) -> Result < Source > {
   67 | let assets = entry [ "assets" ]
   68 | . as_array ( )
   69 | . filter (
```


### 32 normalised tokens: `clean side's lines of playback.rs` lines 69 to 72, and removed@3d7b225 `playback.rs` lines 37 to 41

```text
   69 | )
   70 | . ok_or_else ( || playback_error ( "webPlayback returned no assets" ) ) ? ;
   71 | if let [ only ] = assets . as_slice ( )
   72 | && only [ "flavor" ] . is_null ( )
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 155 to 157, and removed@3d7b225 `lib.rs` lines 13 to 14

```text
  155 | fn next_page_path ( next : & str , first : & str ) -> String {
  156 | let resolved = url :: Url :: parse ( API )
  157 | . ok (
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 27 to 29, and removed@3d7b225 `lib.rs` lines 42 to 44

```text
   27 | )
   28 | && ! name . is_empty ( )
   29 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
```


### 156 normalised tokens: `clean side's lines of lib.rs` lines 90 to 113, and adapted@3d7b225 `lib.rs` lines 947 to 969

```text
   90 | }
  100 | fn apple_id ( id : & str ) -> Result < & str > {
  101 | let catalog = ! id . is_empty ( ) && id . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) ;
  102 | let prefixed = id . split_once ( '.' ) . is_some_and ( | ( kind , name ) | {
  103 | ( 1 ..= 3 ) . contains ( & kind . len ( ) )
  104 | && kind . bytes ( ) . all ( | b | b . is_ascii_lowercase ( ) )
  105 | && ! name . is_empty ( )
  106 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
  107 | } ) ;
  108 | if catalog || prefixed {
  109 | Ok ( id )
  110 | } else {
  111 | Err ( Error :: Protocol ( format ! ( "not an Apple Music id: {id:?}" ) ) )
  112 | }
  113 | }
```


### 141 normalised tokens: `clean side's lines of lib.rs` lines 48 to 73, and adapted@3d7b225 `lib.rs` lines 209 to 234

```text
   48 | reqwest :: Client :: builder ( )
   49 | . user_agent ( USER_AGENT )
   52 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   53 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   56 | . pool_idle_timeout ( std :: time :: Duration :: from_secs ( 300 ) )
   61 | . redirect ( reqwest :: redirect :: Policy :: custom ( | attempt | {
   62 | let apple = attempt
   63 | . url ( )
   64 | . host_str ( )
   65 | . is_some_and ( | host | host == "apple.com" || host . ends_with ( ".apple.com" ) ) ;
   66 | if ! apple {
   67 | attempt . stop ( )
   68 | } else if attempt . previous ( ) . len ( ) > 5 {
   69 | attempt . error ( "too many redirects" )
   70 | } else {
   71 | attempt . follow ( )
   72 | }
   73 | } ) )
```


### 96 normalised tokens: `clean side's lines of playback.rs` lines 265 to 291, and adapted@3d7b225 `playback.rs` lines 762 to 782

```text
  265 | )
  266 | }
  276 | async fn take_licence (
  277 | & self ,
  278 | session : & Session ,
  279 | challenge : & [ u8 ] ,
  280 | adam_id : & str ,
  281 | is_library : bool ,
  282 | key_uri : & str ,
  283 | key_id : & [ u8 ; 16 ] ,
  284 | ) -> Result < ( ) > {
  285 | let licence = self
  286 | . ask_licence ( challenge , adam_id , is_library , key_uri )
  287 | . await ? ;
  288 | session
  289 | . give_licence ( & licence , key_id )
  290 | . map_err ( | e | playback_error ( e . to_string ( ) ) )
  291 | }
```


### 67 normalised tokens: `clean side's lines of lib.rs` lines 287 to 297, and adapted@3d7b225 `lib.rs` lines 414 to 424

```text
  287 | NotSignedIn ) ,
  288 | _ if ! status . is_success ( ) => {
  289 | return Err ( Error :: Protocol ( format ! ( "{path}: HTTP {status}" ) ) ) ;
  290 | }
  291 | _ => { }
  292 | }
  293 | let body = resp
  294 | . text ( )
  295 | . await
  296 | . map_err ( | e | Error :: Network ( format ! ( "{path}: {e}" ) ) ) ? ;
  297 | station :: songs_in (
```


### 60 normalised tokens: `clean side's lines of playback.rs` lines 113 to 126, and adapted@3d7b225 `playback.rs` lines 208 to 214

```text
  113 | }
  123 | fn catalog_file ( playlist_url : & str , body : & [ u8 ] ) -> Option < EncryptedFile > {
  124 | let playlist = m3u8_rs :: parse_media_playlist_res ( body ) . ok ( ) ? ;
  125 | let first = playlist . segments . first ( ) ? ;
  126 | let key = first . key . as_ref ( ) ?
```


### 57 normalised tokens: `clean side's lines of playback.rs` lines 241 to 250, and adapted@3d7b225 `playback.rs` lines 785 to 794

```text
  241 | async fn ask_licence (
  242 | & self ,
  243 | challenge : & [ u8 ] ,
  244 | adam_id : & str ,
  245 | is_library : bool ,
  246 | key_uri : & str ,
  247 | ) -> Result < Vec < u8 >> {
  248 | let body = serde_json :: json ! ( {
  249 | "challenge" : STANDARD . encode ( challenge ) ,
  250 | "key-system" :
```


### 53 normalised tokens: `clean side's lines of lib.rs` lines 257 to 264, and adapted@3d7b225 `lib.rs` lines 1680 to 1684

```text
  257 | , self . storefront ) ;
  258 | let Some ( answer ) = self . fetch :: < types :: LyricsResponse > ( & path ) . await ? else {
  259 | continue ;
  260 | } ;
  261 | let ttml = answer
  262 | . data
  263 | . iter ( )
  264 | . find_map ( | entry | entry . attributes
```


### 34 normalised tokens: `clean side's lines of playback.rs` lines 181 to 186, and adapted@3d7b225 `playback.rs` lines 146 to 151

```text
  181 | resp : reqwest :: Response ,
  182 | again : reqwest :: RequestBuilder ,
  183 | frame : std :: time :: Duration ,
  184 | decrypt : cenc :: Decrypt ,
  185 | ) -> Result < AudioStream > {
  186 | let
```


### 34 normalised tokens: `clean side's lines of playback.rs` lines 250 to 259, and adapted@3d7b225 `playback.rs` lines 794 to 803

```text
  250 | ,
  251 | "uri" : key_uri ,
  252 | "adamId" : adam_id ,
  253 | "isLibrary" : is_library ,
  254 | "user-initiated" : true ,
  255 | } ) ;
  256 | let resp = self
  257 | . send (
  258 | self . http
  259 | . post ( format
```


### 32 normalised tokens: `clean side's lines of convert.rs` lines 78 to 84, and adapted@3d7b225 `convert.rs` lines 413 to 419

```text
   78 | ) ;
   79 | assert ! ( empty_catalog . quality . is_none ( ) ) ;
   80 | }
   82 | # [ test ]
   83 | fn an_album_takes_its_real_genre_its_notes_and_its_catalog_rating ( ) {
   84 | let a = album ( read ( json
```


### 32 normalised tokens: `clean side's lines of playback.rs` lines 55 to 69, and adapted@3d7b225 `playback.rs` lines 91 to 97

```text
   55 | ,
   56 | }
   66 | fn choose_source ( entry : & serde_json :: Value ) -> Result < Source > {
   67 | let assets = entry [ "assets" ]
   68 | . as_array ( )
   69 | . filter (
```


### 32 normalised tokens: `clean side's lines of playback.rs` lines 69 to 72, and adapted@3d7b225 `playback.rs` lines 96 to 100

```text
   69 | )
   70 | . ok_or_else ( || playback_error ( "webPlayback returned no assets" ) ) ? ;
   71 | if let [ only ] = assets . as_slice ( )
   72 | && only [ "flavor" ] . is_null ( )
```


### 32 normalised tokens: `clean side's lines of station.rs` lines 157 to 165, and adapted@3d7b225 `lib.rs` lines 1788 to 1796

```text
  157 | ,
  158 | album : None ,
  159 | album_id : None ,
  160 | duration : None ,
  161 | track_number : None ,
  162 | disc_number : None ,
  163 | artwork : None ,
  164 | explicit : false ,
  165 | quality : None
```


### 31 normalised tokens: `clean side's lines of lib.rs` lines 418 to 421, and adapted@3d7b225 `lib.rs` lines 27 to 30

```text
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json ::
```


### 31 normalised tokens: `clean side's lines of playback.rs` lines 136 to 150, and adapted@3d7b225 `playback.rs` lines 230 to 240

```text
  136 | ,
  137 | keys : vec ! [ Key {
  138 | uri : uri . clone ( ) ,
  139 | id ,
  140 | } ] ,
  141 | } )
  142 | }
  150 | fn licence_in ( answer : & serde_json
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 155 to 157, and adapted@3d7b225 `lib.rs` lines 931 to 932

```text
  155 | fn next_page_path ( next : & str , first : & str ) -> String {
  156 | let resolved = url :: Url :: parse ( API )
  157 | . ok (
```


### 30 normalised tokens: `clean side's lines of lib.rs` lines 417 to 419, and adapted@3d7b225 `convert.rs` lines 5 to 7

```text
  417 | :: test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 27 to 29, and adapted@3d7b225 `lib.rs` lines 960 to 962

```text
   27 | )
   28 | && ! name . is_empty ( )
   29 | && name . bytes ( ) . all ( | b | b . is_ascii_alphanumeric ( ) || b == b'-' )
```


### 131 normalised tokens: `clean side's lines of test_support.rs` lines 579 to 595, and tree `crates/cenc/src/stream.rs` lines 638 to 653

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


### 103 normalised tokens: `clean side's lines of test_support.rs` lines 406 to 422, and tree `crates/cenc/src/tests/build.rs` lines 1695 to 1710

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


### 71 normalised tokens: `clean side's lines of test_support.rs` lines 670 to 678, and tree `crates/cenc/src/stream.rs` lines 862 to 870

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


### 55 normalised tokens: `clean side's lines of station.rs` lines 157 to 174, and tree `crates/connect/src/lib.rs` lines 1433 to 1447

```text
  157 | ,
  158 | album : None ,
  159 | album_id : None ,
  160 | duration : None ,
  161 | track_number : None ,
  162 | disc_number : None ,
  163 | artwork : None ,
  164 | explicit : false ,
  165 | quality : None ,
  166 | in_library : false ,
  167 | library_id : None ,
  168 | }
  169 | }
  174 | fn simulate ( count : usize , per_call : usize ) ->
```


### 54 normalised tokens: `clean side's lines of station.rs` lines 157 to 174, and tree `crates/synced/src/tests.rs` lines 15 to 29

```text
  157 | . into ( ) ] ,
  158 | album : None ,
  159 | album_id : None ,
  160 | duration : None ,
  161 | track_number : None ,
  162 | disc_number : None ,
  163 | artwork : None ,
  164 | explicit : false ,
  165 | quality : None ,
  166 | in_library : false ,
  167 | library_id : None ,
  168 | }
  169 | }
  174 | fn simulate ( count : usize
```


### 47 normalised tokens: `clean side's lines of test_support.rs` lines 626 to 633, and tree `crates/cenc/src/stream.rs` lines 684 to 691

```text
  626 | ) ;
  627 | let mvex = boxed ( b"mvex" , & full_box ( b"trex" , 0 , & trex ) ) ;
  628 | boxed ( b"moov" , & [ trak , mvex ] . concat ( ) )
  629 | }
  633 | fn fragment ( sequence : u32 , count : u32
```


### 46 normalised tokens: `clean side's lines of test_support.rs` lines 644 to 649, and tree `crates/cenc/src/stream.rs` lines 701 to 706

```text
  644 | + 8 + tfhd . len ( ) + 20 + senc . len ( ) ;
  645 | let data_offset = u32 :: try_from ( moof_size + 8 ) . expect ( "a small moof" ) ;
  646 | let trun = full_box (
  647 | b"trun" ,
  648 | 0x1 ,
  649 | & [ count
```


### 44 normalised tokens: `clean side's lines of test_support.rs` lines 186 to 197, and tree `crates/motion/src/lib.rs` lines 56 to 65

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


### 44 normalised tokens: `clean side's lines of test_support.rs` lines 611 to 617, and tree `crates/cenc/src/stream.rs` lines 673 to 679

```text
  611 | ) ;
  612 | let stsd = full_box (
  613 | b"stsd" ,
  614 | 0 ,
  615 | & [ & 1u32 . to_be_bytes ( ) [ .. ] , & boxed ( b"enca" , & enca ) ] . concat ( ) ,
  616 | ) ;
  617 | let mdia = boxed (
```


### 43 normalised tokens: `clean side's lines of lib.rs` lines 48 to 56, and tree `crates/soundcloud/src/lib.rs` lines 149 to 155

```text
   48 | reqwest :: Client :: builder ( )
   49 | . user_agent ( USER_AGENT )
   52 | . connect_timeout ( std :: time :: Duration :: from_secs ( 10 ) )
   53 | . read_timeout ( std :: time :: Duration :: from_secs ( 30 ) )
   56 | . pool_idle_timeout (
```


### 43 normalised tokens: `clean side's lines of test_support.rs` lines 9 to 12, and tree `crates/player/src/engine.rs` lines 5 to 8

```text
    9 | std :: net :: { SocketAddr , TcpListener , TcpStream } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread ::
```


### 40 normalised tokens: `clean side's lines of playback.rs` lines 109 to 113, and tree `crates/widevine/src/find.rs` lines 170 to 174

```text
  109 | if rest . is_empty ( ) || ! rest . bytes ( ) . all ( | b | b . is_ascii_digit ( ) ) {
  110 | return None ;
  111 | }
  112 | rest . parse ( ) . ok ( )
  113 | }
```


### 39 normalised tokens: `clean side's lines of test_support.rs` lines 663 to 665, and tree `crates/cenc/src/stream.rs` lines 849 to 851

```text
  663 | fn encrypted_track ( fragments : u32 , per_fragment : u32 , size : u32 ) -> ( Vec < u8 > , Vec < u8 > ) {
  664 | let mut file = encrypted_movie ( ) ;
  665 | for sequence in
```


### 37 normalised tokens: `clean side's lines of station.rs` lines 33 to 40, and tree `crates/player/src/engine.rs` lines 185 to 199

```text
   33 | count : usize ,
   34 | songs : Vec < Track > ,
   35 | ids : HashSet < String > ,
   36 | sent : usize ,
   37 | in_flight : usize ,
   39 | fruitless : usize ,
   40 | first_failure : Option < Error > ,
```


### 37 normalised tokens: `clean side's lines of test_support.rs` lines 681 to 686, and tree `crates/cenc/src/stream.rs` lines 729 to 734

```text
  681 | Decrypt {
  682 | Box :: new ( | bytes , _ | {
  683 | bytes . iter_mut ( ) . for_each ( | b | * b ^= 0xFF ) ;
  684 | Ok ( ( ) )
  685 | } )
  686 | }
```


### 36 normalised tokens: `clean side's lines of lib.rs` lines 418 to 422, and tree `crates/desktop/src/mpris.rs` lines 12 to 16

```text
  418 | Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
  422 | use std :: sync ::
```


### 35 normalised tokens: `clean side's lines of lib.rs` lines 9 to 22, and tree `crates/app-desktop/src/ui/lyrics.rs` lines 43 to 53

```text
    9 | # [ derive ( Debug , Clone ) ]
   10 | struct Hosts {
   12 | api : String ,
   14 | web_player : String ,
   16 | play : String ,
   18 | licence : String ,
   19 | }
   21 | impl Hosts {
   22 | fn apple (
```


### 35 normalised tokens: `clean side's lines of lib.rs` lines 417 to 421, and tree `crates/desktop/src/backend.rs` lines 10 to 14

```text
  417 | :: test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json ::
```


### 35 normalised tokens: `clean side's lines of test_support.rs` lines 654 to 657, and tree `crates/cenc/src/stream.rs` lines 713 to 716

```text
  654 | ) ;
  655 | let mdat = boxed ( b"mdat" , & vec ! [ 0 ; ( count * size ) as usize ] ) ;
  656 | [ moof , mdat ] . concat ( )
  657 | }
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 10 to 15, and tree `crates/player/src/output.rs` lines 10 to 15

```text
   10 | AtomicBool , AtomicUsize , Ordering } ;
   11 | use std :: sync :: { Arc , Mutex } ;
   12 | use std :: thread :: JoinHandle ;
   13 | use std :: time :: Duration ;
   15 | use base64
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 41 to 51, and tree `crates/cenc/src/tests/build.rs` lines 314 to 323

```text
   41 | ,
   43 | pub host : String ,
   45 | pub headers : Vec < ( String , String ) > ,
   46 | pub body : Vec < u8 > ,
   47 | }
   49 | impl Request {
   51 | pub fn header (
```


### 34 normalised tokens: `clean side's lines of test_support.rs` lines 189 to 197, and tree `crates/player/src/engine.rs` lines 88 to 95

```text
  189 | {
  190 | self . requests
  191 | . lock ( )
  192 | . unwrap_or_else ( | e | e . into_inner ( ) )
  193 | . clone ( )
  194 | }
  197 | pub fn requests_to ( & self , path :
```


### 33 normalised tokens: `clean side's lines of lib.rs` lines 416 to 419, and tree `crates/opium-core/src/provider.rs` lines 1 to 5

```text
  416 | ;
  417 | use crate :: test_support :: {
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
```


### 33 normalised tokens: `clean side's lines of lib.rs` lines 418 to 421, and tree `crates/opium-ffi/src/lib.rs` lines 10 to 13

```text
  418 | Reply , Request , TestServer , USER_TOKEN , apple_music , apple_server , fake_token , json_reply ,
  419 | lasting_token , reply , seconds_from_now , test_client , web_player_and_storefront ,
  420 | } ;
  421 | use serde_json :: json ;
```


### 33 normalised tokens: `clean side's lines of test_support.rs` lines 149 to 151, and tree `crates/player/src/lib.rs` lines 370 to 372

```text
  149 | port ( ) ;
  150 | let requests = Arc :: new ( Mutex :: new ( Vec :: new ( ) ) ) ;
  151 | let peak = Arc :: new ( AtomicUsize :: new
```


### 33 normalised tokens: `clean side's lines of test_support.rs` lines 597 to 605, and tree `crates/cenc/src/stream.rs` lines 660 to 665

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


### 33 normalised tokens: `clean side's lines of test_support.rs` lines 669 to 671, and tree `crates/cenc/fuzz/fuzz_targets/stream_download.rs` lines 57 to 59

```text
  669 | let mut clear = file . clone ( ) ;
  670 | cenc :: make_clear ( & mut clear , & movie , & samples ) ;
  671 | for sample in samples . iter ( )
```


### 32 normalised tokens: `clean side's lines of playback.rs` lines 101 to 108, and tree `crates/lyrics/src/lrc.rs` lines 24 to 29

```text
  101 | url . is_empty ( ) )
  102 | }
  107 | fn ctr_kbps ( flavor : & str ) -> Option < u32 > {
  108 | let ( _ , rest ) = flavor . split_once (
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 51 to 55, and tree `crates/downloads/src/queue.rs` lines 73 to 77

```text
   51 | {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( | ( n , _ ) | n == name )
   55 | . map ( | ( _ , v ) |
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 456 to 457, and tree `crates/widevine/src/tests/find.rs` lines 337 to 338

```text
  456 | fn mutate_bytes ( rng : & mut Rng , seed : & [ u8 ] ) -> Vec < u8 > {
  457 | let mut out = seed . to_vec ( ) ;
```


### 32 normalised tokens: `clean side's lines of test_support.rs` lines 496 to 498, and tree `crates/cenc/src/tests/build.rs` lines 1749 to 1751

```text
  496 | ) . map ( | _ | rng . next_u64 ( ) as u8 ) . collect ( ) ;
  497 | out . splice ( i .. i , random ) ;
  498 | }
```


### 31 normalised tokens: `clean side's lines of playback.rs` lines 1063 to 1064, and tree `crates/app-desktop/src/ui/playlist_dialog.rs` lines 262 to 267

```text
 1063 | = file . keys . iter ( ) . map ( | k | k . uri . as_str ( ) ) . collect ( ) ;
 1064 | uris . dedup (
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 37 to 45, and tree `crates/app-desktop/src/ui/menu.rs` lines 402 to 412

```text
   37 | Debug , Clone ) ]
   38 | pub ( crate ) struct Request {
   39 | pub method : String ,
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec
```


### 31 normalised tokens: `clean side's lines of test_support.rs` lines 41 to 47, and tree `crates/soundcloud/src/lib.rs` lines 69 to 76

```text
   41 | pub path : String ,
   43 | pub host : String ,
   45 | pub headers : Vec < ( String , String ) > ,
   46 | pub body : Vec < u8 > ,
   47 | }
```


### 30 normalised tokens: `clean side's lines of playback.rs` lines 560 to 564, and tree `crates/local/src/lib.rs` lines 15 to 19

```text
  560 | Reply , Request , USER_TOKEN , apple_music , apple_server , encrypted_track , json_reply , reply ,
  561 | xor_decrypt ,
  562 | } ;
  563 | use opium_core :: model :: ProviderKind ;
  564 | use serde_json ::
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 51 to 54, and tree `crates/app-desktop/src/app.rs` lines 2850 to 2853

```text
   51 | pub fn header ( & self , name : & str ) -> Option < & str > {
   52 | self . headers
   53 | . iter ( )
   54 | . find ( |
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 224 to 229, and tree `crates/player/src/output.rs` lines 203 to 208

```text
  224 | ) ) ;
  225 | if let Some ( accepting ) = self . accepting . take ( ) {
  226 | let _ = accepting . join ( ) ;
  227 | }
  228 | }
  229 | }
```


### 30 normalised tokens: `clean side's lines of test_support.rs` lines 624 to 626, and tree `crates/cenc/src/stream.rs` lines 680 to 682

```text
  624 | ) ;
  625 | let trak = boxed ( b"trak" , & [ full_box ( b"tkhd" , 7 , & tkhd ) , mdia ] . concat ( ) ) ;
  626 | let
```


## Names, strings and comments of the sources found in the new code


### kopuz-a58c385 api.rs: `api.rs`

- Compound identifiers of 6 characters or more: 125; found in the new code: 32.
  `DeserializeOwned` (convert.rs), `HashSet` (lib.rs, mutation_tests.rs, station.rs), `USER_AGENT` (lib.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_array` (playback.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_ref` (lib.rs, playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_u16` (lib.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `from_str` (station.rs), `from_value` (convert.rs), `futures_util` (lib.rs), `into_iter` (auth.rs, lib.rs, station.rs, test_support.rs), `into_owned` (auth.rs, lib.rs, test_support.rs), `is_ascii_digit` (lib.rs, mutation_tests.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `is_success` (auth.rs, lib.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `map_or` (auth.rs, lib.rs), `ok_or_else` (auth.rs, playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `starts_with` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `strip_prefix` (auth.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs)
- String literals of three words or more: 116; found: 0.
- Comment phrases of six words: 787; found: 0.

### kopuz-a58c385 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 11; found in the new code: 4.
  `USER_AGENT` (lib.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs)
- String literals of three words or more: 21; found: 0.
- Comment phrases of six words: 0; found: 0.

### kopuz-a58c385 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 18.
  `PartialEq` (playback.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_ref` (lib.rs, playback.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `checked_add` (auth.rs, playback.rs), `extend_from_slice` (auth.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `key_id` (playback.rs), `saturating_sub` (station.rs, test_support.rs), `to_be_bytes` (test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_vec` (playback.rs, test_support.rs), `try_into` (playback.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### kopuz-a58c385 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 57; found in the new code: 19.
  `AppleMusic` (convert.rs, lib.rs, playback.rs, station.rs, test_support.rs), `adam_id` (playback.rs), `album_id` (convert.rs, station.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `artist_id` (convert.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_ref` (lib.rs, playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `disc_number` (convert.rs, station.rs), `from_value` (convert.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `track_number` (convert.rs, station.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 4; found: 0.
- Comment phrases of six words: 130; found: 0.

### kopuz-a58c385 progressive.rs: `progressive.rs`

- Compound identifiers of 6 characters or more: 83; found in the new code: 17.
  `AtomicUsize` (lib.rs, test_support.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_ref` (lib.rs, playback.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `from_millis` (auth.rs, convert.rs, lib.rs), `from_secs` (auth.rs, lib.rs, test_support.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_err` (playback.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `key_id` (playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `read_to_end` (playback.rs), `saturating_sub` (station.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs)
- String literals of three words or more: 34; found: 0.
- Comment phrases of six words: 806; found: 0.

### kopuz-a58c385 signin.rs: `signin.rs`

- Compound identifiers of 6 characters or more: 40; found in the new code: 11.
  `as_secs` (test_support.rs), `from_millis` (auth.rs, convert.rs, lib.rs), `into_iter` (auth.rs, lib.rs, station.rs, test_support.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `ok_or_else` (auth.rs, playback.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 11; found: 0.
- Comment phrases of six words: 39; found: 0.

### kopuz-a58c385 stream.rs: `stream.rs`

- Compound identifiers of 6 characters or more: 131; found in the new code: 42.
  `PartialEq` (playback.rs), `USER_AGENT` (lib.rs), `adam_id` (playback.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_array` (playback.rs), `as_bytes` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_i64` (auth.rs), `as_object` (playback.rs), `as_ref` (lib.rs, playback.rs), `as_slice` (playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `asset_url` (playback.rs), `content_length` (playback.rs), `from_slice` (auth.rs, mutation_tests.rs, test_support.rs), `from_str` (station.rs), `general_purpose` (auth.rs, lib.rs, test_support.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_err` (playback.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `is_success` (auth.rs, lib.rs), `key_id` (playback.rs), `key_uri` (playback.rs), `m3u8_rs` (playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `media_playlist` (playback.rs), `ok_or_else` (auth.rs, playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `spawn_blocking` (playback.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `starts_with` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `then_some` (playback.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_vec` (playback.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs), `without_url` (auth.rs), `write_all` (test_support.rs)
- String literals of three words or more: 78; found: 0.
- Comment phrases of six words: 595; found: 0.

### kopuz-a58c385 types.rs: `types.rs`

- Compound identifiers of 6 characters or more: 76; found in the new code: 5.
  `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `from_value` (convert.rs), `is_library` (playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 78; found: 0.

### kopuz-1d8b323 api.rs: `api.rs`

- Compound identifiers of 6 characters or more: 125; found in the new code: 32.
  `DeserializeOwned` (convert.rs), `HashSet` (lib.rs, mutation_tests.rs, station.rs), `USER_AGENT` (lib.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_array` (playback.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_ref` (lib.rs, playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_u16` (lib.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `from_str` (station.rs), `from_value` (convert.rs), `futures_util` (lib.rs), `into_iter` (auth.rs, lib.rs, station.rs, test_support.rs), `into_owned` (auth.rs, lib.rs, test_support.rs), `is_ascii_digit` (lib.rs, mutation_tests.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `is_success` (auth.rs, lib.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `map_or` (auth.rs, lib.rs), `ok_or_else` (auth.rs, playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `starts_with` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `strip_prefix` (auth.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs)
- String literals of three words or more: 116; found: 0.
- Comment phrases of six words: 787; found: 0.

### kopuz-1d8b323 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 11; found in the new code: 4.
  `USER_AGENT` (lib.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs)
- String literals of three words or more: 21; found: 0.
- Comment phrases of six words: 0; found: 0.

### kopuz-1d8b323 cenc.rs: `cenc.rs`

- Compound identifiers of 6 characters or more: 135; found in the new code: 18.
  `PartialEq` (playback.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_ref` (lib.rs, playback.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `checked_add` (auth.rs, playback.rs), `extend_from_slice` (auth.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `key_id` (playback.rs), `saturating_sub` (station.rs, test_support.rs), `to_be_bytes` (test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_vec` (playback.rs, test_support.rs), `try_into` (playback.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 23; found: 0.
- Comment phrases of six words: 408; found: 0.

### kopuz-1d8b323 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 57; found in the new code: 19.
  `AppleMusic` (convert.rs, lib.rs, playback.rs, station.rs, test_support.rs), `adam_id` (playback.rs), `album_id` (convert.rs, station.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `artist_id` (convert.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_ref` (lib.rs, playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `disc_number` (convert.rs, station.rs), `from_value` (convert.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `track_number` (convert.rs, station.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 4; found: 0.
- Comment phrases of six words: 130; found: 0.

### kopuz-1d8b323 progressive.rs: `progressive.rs`

- Compound identifiers of 6 characters or more: 83; found in the new code: 17.
  `AtomicUsize` (lib.rs, test_support.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_ref` (lib.rs, playback.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `from_millis` (auth.rs, convert.rs, lib.rs), `from_secs` (auth.rs, lib.rs, test_support.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_err` (playback.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `key_id` (playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `read_to_end` (playback.rs), `saturating_sub` (station.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs)
- String literals of three words or more: 34; found: 0.
- Comment phrases of six words: 806; found: 0.

### kopuz-1d8b323 signin.rs: `signin.rs`

- Compound identifiers of 6 characters or more: 40; found in the new code: 11.
  `as_secs` (test_support.rs), `from_millis` (auth.rs, convert.rs, lib.rs), `into_iter` (auth.rs, lib.rs, station.rs, test_support.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `ok_or_else` (auth.rs, playback.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs)
- String literals of three words or more: 11; found: 0.
- Comment phrases of six words: 39; found: 0.

### kopuz-1d8b323 stream.rs: `stream.rs`

- Compound identifiers of 6 characters or more: 131; found in the new code: 42.
  `PartialEq` (playback.rs), `USER_AGENT` (lib.rs), `adam_id` (playback.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_array` (playback.rs), `as_bytes` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_i64` (auth.rs), `as_object` (playback.rs), `as_ref` (lib.rs, playback.rs), `as_slice` (playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `asset_url` (playback.rs), `content_length` (playback.rs), `from_slice` (auth.rs, mutation_tests.rs, test_support.rs), `from_str` (station.rs), `general_purpose` (auth.rs, lib.rs, test_support.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_err` (playback.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `is_success` (auth.rs, lib.rs), `key_id` (playback.rs), `key_uri` (playback.rs), `m3u8_rs` (playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `media_playlist` (playback.rs), `ok_or_else` (auth.rs, playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `spawn_blocking` (playback.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `starts_with` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `then_some` (playback.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `to_vec` (playback.rs, test_support.rs), `unwrap_or` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_default` (test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs), `without_url` (auth.rs), `write_all` (test_support.rs)
- String literals of three words or more: 78; found: 0.
- Comment phrases of six words: 595; found: 0.

### kopuz-1d8b323 types.rs: `types.rs`

- Compound identifiers of 6 characters or more: 76; found in the new code: 5.
  `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `catalog_id` (convert.rs), `from_value` (convert.rs), `is_library` (playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 78; found: 0.

### removed@3d7b225 auth.rs: `auth.rs`

- Compound identifiers of 6 characters or more: 10; found in the new code: 9.
  `LazyLock` (auth.rs), `WEB_PLAYER` (auth.rs, lib.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `developer_token` (auth.rs, lib.rs), `is_success` (auth.rs, lib.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `ok_or_else` (auth.rs, playback.rs), `opium_core` (auth.rs, playback.rs, station.rs), `to_owned` (auth.rs, lib.rs, playback.rs, test_support.rs)
- String literals of three words or more: 3; found: 0.
- Comment phrases of six words: 39; found: 0.

### removed@3d7b225 convert.rs: `convert.rs`

- Compound identifiers of 6 characters or more: 7; found in the new code: 4.
  `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `from_secs` (auth.rs, lib.rs, test_support.rs), `is_some` (auth.rs, convert.rs, lib.rs, playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs)
- String literals of three words or more: 1; found: 0.
- Comment phrases of six words: 0; found: 0.

### removed@3d7b225 lib.rs: `lib.rs`

- Compound identifiers of 6 characters or more: 34; found in the new code: 25.
  `LyricsResponse` (lib.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `apple_id` (lib.rs, mutation_tests.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `at_offset` (lib.rs, mutation_tests.rs), `ends_with` (lib.rs), `find_map` (lib.rs), `get_or_insert` (station.rs), `into_iter` (auth.rs, lib.rs, station.rs, test_support.rs), `into_owned` (auth.rs, lib.rs, test_support.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_ascii_digit` (lib.rs, mutation_tests.rs, playback.rs), `is_ascii_lowercase` (lib.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_err` (playback.rs), `is_some_and` (auth.rs, lib.rs, playback.rs), `is_success` (auth.rs, lib.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `parse_ttml` (lib.rs), `query_pairs` (lib.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `to_owned` (auth.rs, lib.rs, playback.rs, test_support.rs), `unwrap_or_else` (lib.rs, station.rs, test_support.rs), `user_token` (lib.rs, playback.rs)
- String literals of three words or more: 3; found: 2.
  - `"not an Apple Music id: {id:?}"`
  - `"the station offered no songs"`
- Comment phrases of six words: 107; found: 0.

### removed@3d7b225 playback.rs: `playback.rs`

- Compound identifiers of 6 characters or more: 49; found in the new code: 35.
  `AudioStream` (playback.rs), `EncryptedFile` (playback.rs), `PartialEq` (playback.rs), `adam_id` (playback.rs), `and_then` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `as_array` (playback.rs), `as_deref` (convert.rs, lib.rs, playback.rs), `as_i64` (auth.rs), `as_ref` (lib.rs, playback.rs), `as_slice` (playback.rs), `as_str` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, test_support.rs), `assert_eq` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `filter_map` (lib.rs, playback.rs), `give_licence` (playback.rs), `is_ascii_alphanumeric` (auth.rs, lib.rs, playback.rs), `is_empty` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `is_library` (playback.rs), `is_none` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs), `json_body` (playback.rs), `key_id` (playback.rs), `key_uri` (playback.rs), `m3u8_rs` (playback.rs), `map_err` (auth.rs, lib.rs, playback.rs, station.rs), `map_or` (auth.rs, lib.rs), `ok_or_else` (auth.rs, playback.rs), `parse_media_playlist_res` (playback.rs), `playback_error` (playback.rs), `playlist_url` (playback.rs), `serde_json` (auth.rs, convert.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `split_once` (lib.rs, playback.rs, test_support.rs), `starts_with` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs), `to_owned` (auth.rs, lib.rs, playback.rs, test_support.rs), `to_string` (auth.rs, lib.rs, mutation_tests.rs, playback.rs, station.rs, test_support.rs), `try_into` (playback.rs), `user_token` (lib.rs, playback.rs)
- String literals of three words or more: 6; found: 4.
  - `"licence refused (errorCode {code})"`
  - `"licence response without a licence"`
  - `"no Widevine asset (only FairPlay's)"`
  - `"webPlayback returned no assets"`
- Comment phrases of six words: 72; found: 0.
