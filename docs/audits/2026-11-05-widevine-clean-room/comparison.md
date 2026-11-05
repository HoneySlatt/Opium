# Token comparison

Targets:

- `crates/widevine/build.rs`
- `crates/widevine/examples/probe.rs`
- `crates/widevine/shim/boundary.cc`
- `crates/widevine/shim/boundary.h`
- `crates/widevine/shim/cdm_11.cc`
- `crates/widevine/src/android.rs`
- `crates/widevine/src/boundary.rs`
- `crates/widevine/src/desktop.rs`
- `crates/widevine/src/error.rs`
- `crates/widevine/src/find.rs`
- `crates/widevine/src/host.rs`
- `crates/widevine/src/library.rs`
- `crates/widevine/src/lib.rs`
- `crates/widevine/src/log.rs`
- `crates/widevine/src/manifest.rs`
- `crates/widevine/src/pssh.rs`
- `crates/widevine/src/runtime.rs`
- `crates/widevine/src/tests/cpp_layer.rs`
- `crates/widevine/src/tests/fake.rs`
- `crates/widevine/src/tests/find.rs`
- `crates/widevine/src/tests/host.rs`
- `crates/widevine/src/tests/loader.rs`
- `crates/widevine/src/tests/mod.rs`
- `crates/widevine/src/tests/round_trip.rs`
- `crates/widevine/src/tests/sequence.rs`
- `crates/widevine/src/timers.rs`

Sources:

- kopuz-a58c385: 764 files
- kopuz-1d8b323: 771 files
- adapted@fce9347: 6 files
- tree: 152 files

## Raw tokens, runs of 50 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 0 | - | 42 |
| kopuz-1d8b323 | 771 | 0 | - | 42 |
| adapted@fce9347 | 6 | 0 | - | 46 |
| tree | 152 | 0 | - | 30 |

## Normalised tokens, runs of 30 or more

| Against | Files | Runs at or above | Longest at or above | Longest below |
|---|---|---|---|---|
| kopuz-a58c385 | 764 | 105 | 119 | 29 |
| kopuz-1d8b323 | 771 | 105 | 119 | 29 |
| adapted@fce9347 | 6 | 9 | 101 | 29 |
| tree | 152 | 59 | 54 | 29 |

### 119 normalised tokens: `crates/widevine/src/pssh.rs` lines 36 to 53, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/mod.rs` lines 463 to 482

```text
   36 | + data . len ( ) ;
   37 | let mut pssh = Vec :: with_capacity ( size ) ;
   39 | pssh . extend_from_slice ( & ( size as u32 ) . to_be_bytes ( ) ) ;
   40 | pssh . extend_from_slice ( b"pssh" ) ;
   41 | pssh . extend_from_slice ( & [ 0 , 0 , 0 , 0 ] ) ;
   42 | pssh . extend_from_slice ( & SYSTEM_ID ) ;
   43 | pssh . extend_from_slice ( & ( data . len ( ) as u32 ) . to_be_bytes ( ) ) ;
   44 | pssh . extend_from_slice ( & data ) ;
   45 | pssh
   46 | }
   48 | # [ cfg ( test ) ]
   49 | mod tests {
   50 | use super :: * ;
   52 | # [ test ]
   53 | fn the_pssh_for_a_key_id_is_pinned ( ) {
```


### 56 normalised tokens: `crates/widevine/src/tests/host.rs` lines 134 to 141, and kopuz-a58c385 `daemon/server/src/ytmusic/fixture_tests.rs` lines 768 to 775

```text
  134 | ) ;
  135 | let creates = calls
  136 | . iter ( )
  137 | . filter ( | call | matches ! ( call , Call :: Create { .. } ) )
  138 | . count ( ) ;
  139 | let destroys = calls
  140 | . iter ( )
  141 | . filter ( | call | matches ! ( call , Call :: Destroy
```


### 55 normalised tokens: `crates/widevine/src/host.rs` lines 222 to 230, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 803 to 811

```text
  222 | ,
  223 | inbox : Inbox :: for_this_thread ( ) ,
  224 | instance : None ,
  225 | timers : Timers :: default ( ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise :
```


### 49 normalised tokens: `crates/widevine/src/tests/find.rs` lines 82 to 88, and kopuz-a58c385 `daemon/db/src/backend/queries.rs` lines 1176 to 1182

```text
   82 | }
   84 | fn find < 'a > ( candidates : & 'a [ Candidate ] , path : & Path ) -> & 'a Candidate {
   85 | candidates
   86 | . iter ( )
   87 | . find ( | candidate | candidate . path == path )
   88 | . unwrap_or_else ( || panic ! (
```


### 46 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 215, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 378 to 389

```text
  199 | struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
  215 | impl Shared
```


### 45 normalised tokens: `crates/widevine/src/pssh.rs` lines 4 to 6, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/mod.rs` lines 24 to 26

```text
    4 | pub const SYSTEM_ID : [ u8 ; 16 ] = [
    5 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d , 0x21 , 0xed ,
    6 | ] ;
```


### 45 normalised tokens: `crates/widevine/src/tests/host.rs` lines 13 to 21, and kopuz-a58c385 `apps/kopuz/components/src/carousel.rs` lines 9 to 17

```text
   13 | MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
   15 | } ;
   16 | use crate :: runtime :: { Deadlines , Runtime } ;
   17 | use crate :: { Error , Protected , Scheme , pssh_for_key } ;
   21 | const HANG_GUARD : Duration =
```


### 45 normalised tokens: `crates/widevine/src/tests/sequence.rs` lines 13 to 18, and kopuz-a58c385 `apps/kopuz/components/src/track_meta.rs` lines 16 to 21

```text
   13 | key , start } ;
   14 | use crate :: boundary :: { MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST } ;
   15 | use crate :: runtime :: { Deadlines , Runtime } ;
   16 | use crate :: { Error , Protected , Scheme , pssh_for_key } ;
   18 | const SEQUENCES : u64 =
```


### 43 normalised tokens: `crates/widevine/src/boundary.rs` lines 210 to 226, and kopuz-a58c385 `shared/config/src/ui_prefs.rs` lines 8 to 14

```text
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
  223 | } ;
  226 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
```


### 43 normalised tokens: `crates/widevine/src/host.rs` lines 202 to 208, and kopuz-a58c385 `apps/kopuz/components/src/steps.rs` lines 303 to 309

```text
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
```


### 43 normalised tokens: `crates/widevine/src/host.rs` lines 203 to 210, and kopuz-a58c385 `apps/kopuz/components/src/folders.rs` lines 176 to 183

```text
  203 | ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32
```


### 42 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 405 to 416, and kopuz-a58c385 `apps/legacy/kopuz/src/main.rs` lines 1536 to 1547

```text
  405 | ,
  406 | create : fake_create ,
  407 | initialize : fake_initialize ,
  408 | create_session : fake_create_session ,
  409 | update_session : fake_update_session ,
  410 | close_session : fake_close_session ,
  411 | timer_expired : fake_timer_expired ,
  412 | decrypt : fake_decrypt ,
  413 | release_buffer : fake_release_buffer ,
  414 | storage_id : fake_storage_id ,
  415 | destroy : fake_destroy ,
  416 | }
```


### 41 normalised tokens: `crates/widevine/src/find.rs` lines 139 to 143, and kopuz-a58c385 `apps/legacy/hooks/src/use_player_controller.rs` lines 173 to 177

```text
  139 | fn choose ( candidates : & [ Candidate ] ) -> Option < PathBuf > {
  140 | candidates
  141 | . iter ( )
  142 | . filter ( | candidate | candidate . set_aside . is_none ( ) )
  143 | . map ( | candidate |
```


### 41 normalised tokens: `crates/widevine/src/lib.rs` lines 52 to 59, and kopuz-a58c385 `shared/api/src/lib.rs` lines 77 to 83

```text
   52 | find :: { cdm_candidates , find_cdm } ;
   53 | pub use pssh :: { SYSTEM_ID , pssh_for_key } ;
   56 | # [ derive ( Debug , Clone , PartialEq , Eq ) ]
   57 | pub struct Candidate {
   59 | pub path : std :: path
```


### 40 normalised tokens: `crates/widevine/src/host.rs` lines 204 to 212, and kopuz-a58c385 `apps/kopuz/palette/src/lib.rs` lines 143 to 153

```text
  204 | ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec
```


### 40 normalised tokens: `crates/widevine/src/host.rs` lines 225 to 231, and kopuz-a58c385 `daemon/discord-presence/src/cover_art.rs` lines 295 to 301

```text
  225 | ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise : 0 ,
  231 | subsamples :
```


### 40 normalised tokens: `crates/widevine/src/lib.rs` lines 52 to 59, and kopuz-a58c385 `apps/kopuz/theme/src/lib.rs` lines 19 to 25

```text
   52 | pub use find :: { cdm_candidates , find_cdm } ;
   53 | pub use pssh :: { SYSTEM_ID , pssh_for_key } ;
   56 | # [ derive ( Debug , Clone , PartialEq , Eq ) ]
   57 | pub struct Candidate {
   59 | pub path :
```


### 40 normalised tokens: `crates/widevine/src/runtime.rs` lines 203 to 216, and kopuz-a58c385 `apps/kopuz/components/src/transport.rs` lines 38 to 49

```text
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
  215 | impl Shared {
  216 | pub
```


### 40 normalised tokens: `crates/widevine/src/tests/host.rs` lines 4 to 8, and kopuz-a58c385 `daemon/core/src/offline.rs` lines 4 to 9

```text
    4 | sync :: atomic :: Ordering ;
    5 | use std :: sync :: mpsc :: { Receiver , channel } ;
    6 | use std :: sync :: { Arc , Mutex } ;
    7 | use std :: thread ;
    8 | use std :: time ::
```


### 40 normalised tokens: `crates/widevine/src/timers.rs` lines 149 to 156, and kopuz-a58c385 `apps/kopuz/primitives/src/motion.rs` lines 414 to 421

```text
  149 | ) ) ;
  150 | }
  152 | # [ test ]
  153 | fn negative_and_huge_delays ( ) {
  154 | let start = Instant :: now ( ) ;
  155 | let mut timers = Timers :: default ( ) ;
  156 | assert ! ( timers . set ( start
```


### 38 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 49 to 52, and kopuz-a58c385 `daemon/server/shim/widevine_shim.cc` lines 268 to 271

```text
   49 | capacity_ ; }
   50 | uint8_t * Data ( ) override { return data_ ; }
   51 | void SetSize ( uint32_t size ) override { size_ = size ; }
   52 | uint32_t Size ( ) const override { return size_ ; }
```


### 38 normalised tokens: `crates/widevine/src/desktop.rs` lines 118 to 125, and kopuz-a58c385 `apps/kopuz/store/src/target.rs` lines 65 to 71

```text
  118 | key ) ;
  119 | }
  120 | }
  122 | impl fmt :: Debug for Session {
  123 | fn fmt ( & self , formatter : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
  124 | formatter
  125 | . debug_struct (
```


### 38 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 36 to 45, and kopuz-a58c385 `daemon/server/src/ytmusic/discover.rs` lines 1338 to 1347

```text
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec < u8 > ,
   39 | response : Vec < u8 > ,
   40 | } ,
   41 | CloseSession {
   42 | promise : u32 ,
   43 | session : Vec < u8 > ,
   44 | } ,
   45 | Decrypt
```


### 38 normalised tokens: `crates/widevine/src/tests/host.rs` lines 953 to 959, and kopuz-a58c385 `daemon/server/src/ytmusic/fixture_tests.rs` lines 562 to 568

```text
  953 | )
  954 | . filter_map ( | call | match call {
  955 | Call :: CloseSession { session , .. } => Some ( session ) ,
  956 | _ => None ,
  957 | } )
  958 | . collect ( ) ;
  959 | assert_eq ! (
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 196 to 205, and kopuz-a58c385 `apps/kopuz/shortcuts/src/editor.rs` lines 18 to 26

```text
  196 | ,
  197 | }
  200 | pub ( crate ) struct Host {
  201 | shared : Arc < Shared > ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 203 to 209, and kopuz-a58c385 `apps/kopuz/page_albums/src/detail.rs` lines 50 to 57

```text
  203 | > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 205 to 212, and kopuz-a58c385 `apps/kopuz/metadata_dialog/src/lib.rs` lines 74 to 80

```text
  205 | ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec <
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 1176 to 1178, and kopuz-a58c385 `apps/kopuz/store/src/jobs.rs` lines 202 to 204

```text
 1176 | fn session ( & self , session : SessionKey ) -> Option < & SessionEntry > {
 1177 | self . sessions . iter ( ) . find ( | entry | entry . key == session )
 1178 | }
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 1177 to 1181, and kopuz-a58c385 `apps/kopuz/discover/src/shelves.rs` lines 164 to 170

```text
 1177 | )
 1178 | }
 1180 | fn session_mut ( & mut self , session : SessionKey ) -> Option < & mut SessionEntry > {
 1181 | self . sessions . iter_mut ( ) . find ( | entry | entry . key
```


### 37 normalised tokens: `crates/widevine/src/lib.rs` lines 59 to 68, and kopuz-a58c385 `shared/api/src/sources.rs` lines 270 to 277

```text
   59 | PathBuf ,
   61 | pub version : Option < String > ,
   63 | pub set_aside : Option < String > ,
   64 | }
   67 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
   68 | pub struct Protected
```


### 37 normalised tokens: `crates/widevine/src/timers.rs` lines 116 to 122, and kopuz-a58c385 `apps/kopuz/primitives/src/motion.rs` lines 415 to 421

```text
  116 | }
  118 | # [ test ]
  119 | fn timers_fire_in_order_of_due_time_and_never_before ( ) {
  120 | let start = Instant :: now ( ) ;
  121 | let mut timers = Timers :: default ( ) ;
  122 | assert ! ( timers . set ( start
```


### 36 normalised tokens: `crates/widevine/src/boundary.rs` lines 13 to 16, and kopuz-a58c385 `daemon/core/src/config_service.rs` lines 3 to 7

```text
   13 | use std :: cell :: { Cell , RefCell } ;
   14 | use std :: ffi :: { c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std ::
```


### 36 normalised tokens: `crates/widevine/src/desktop.rs` lines 145 to 155, and kopuz-a58c385 `daemon/core/src/spotify.rs` lines 39 to 46

```text
  145 | loading : Mutex < Loaded > ,
  147 | current : Mutex < Current > ,
  148 | }
  150 | # [ derive ( Default ) ]
  151 | struct Loaded {
  153 | path : Option < PathBuf > ,
  155 | ready : bool ,
```


### 36 normalised tokens: `crates/widevine/src/find.rs` lines 187 to 193, and kopuz-a58c385 `daemon/core/src/url_download.rs` lines 777 to 782

```text
  187 | )
  188 | . find ( | ordering | ordering . is_ne ( ) )
  189 | . unwrap_or ( Ordering :: Equal )
  190 | }
  193 | fn chromium_trees ( config : & Path , platform : & str , found :
```


### 36 normalised tokens: `crates/widevine/src/host.rs` lines 206 to 214, and kopuz-a58c385 `apps/kopuz/components/src/source_card.rs` lines 20 to 26

```text
  206 | ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32
```


### 36 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 209, and kopuz-a58c385 `daemon/server/src/nextcloud/tree.rs` lines 43 to 49

```text
  199 | pub ( crate ) struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64
```


### 36 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 9 to 12, and kopuz-a58c385 `apps/kopuz/components/src/settings.rs` lines 6 to 9

```text
    9 | c_char , c_void } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicI64 , Ordering } ;
   11 | use std :: sync :: mpsc :: Sender ;
   12 | use std :: sync :: { Arc
```


### 36 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 31 to 35, and kopuz-a58c385 `daemon/server/src/applemusic/stream.rs` lines 884 to 886

```text
   31 | ,
   32 | std :: process :: id ( )
   33 | ) ) ;
   34 | let _ = std :: fs :: remove_dir_all ( & path ) ;
   35 | std :: fs :: create_dir_all ( & path ) . expect (
```


### 36 normalised tokens: `crates/widevine/src/timers.rs` lines 137 to 144, and kopuz-a58c385 `apps/kopuz/primitives/src/motion.rs` lines 428 to 435

```text
  137 | ) ) ;
  138 | }
  140 | # [ test ]
  141 | fn a_timer_set_during_a_round_waits_for_the_next ( ) {
  142 | let start = Instant :: now ( ) ;
  143 | let mut timers = Timers :: default ( ) ;
  144 | timers . set (
```


### 35 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 72, and kopuz-a58c385 `daemon/server/shim/widevine_shim.cc` lines 269 to 271

```text
   70 | Buffer * DecryptedBuffer ( ) override { return buffer_ ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
```


### 35 normalised tokens: `crates/widevine/src/host.rs` lines 226 to 231, and kopuz-a58c385 `daemon/core/src/artwork.rs` lines 643 to 648

```text
  226 | new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise : 0 ,
  231 | subsamples : Vec
```


### 35 normalised tokens: `crates/widevine/src/pssh.rs` lines 58 to 61, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/mod.rs` lines 509 to 512

```text
   58 | ] ,
   59 | & [
   60 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d ,
   61 | 0x21 , 0xed
```


### 35 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 28 to 38, and kopuz-a58c385 `daemon/server/src/catalog.rs` lines 31 to 41

```text
   28 | ,
   29 | } ,
   30 | CreateSession {
   31 | promise : u32 ,
   32 | session_type : u32 ,
   33 | init_data_type : u32 ,
   34 | init_data : Vec < u8 > ,
   35 | } ,
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec
```


### 35 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 57 to 62, and kopuz-a58c385 `daemon/server/src/ytmusic/search.rs` lines 41 to 46

```text
   57 | ) struct DecryptCall {
   58 | pub size : usize ,
   59 | pub scheme : u32 ,
   60 | pub key_id : Vec < u8 > ,
   61 | pub iv : Vec < u8 > ,
   62 | pub subsamples : Vec <
```


### 34 normalised tokens: `crates/widevine/src/boundary.rs` lines 209 to 226, and kopuz-a58c385 `daemon/player/src/player.rs` lines 12 to 16

```text
  209 | {
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
  223 | } ;
  226 | # [ derive ( Debug
```


### 34 normalised tokens: `crates/widevine/src/find.rs` lines 68 to 76, and kopuz-a58c385 `daemon/db/src/backend/rows.rs` lines 26 to 30

```text
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf ,
   76 | pub platform : Option <
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 11 to 16, and kopuz-a58c385 `apps/kopuz/onboarding/src/ui.rs` lines 4 to 9

```text
   11 | use std :: collections :: VecDeque ;
   12 | use std :: ffi :: c_void ;
   13 | use std :: fmt ;
   14 | use std :: ptr ;
   15 | use std :: sync :: Arc ;
   16 | use std ::
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 201 to 207, and kopuz-a58c385 `daemon/core/src/spotify.rs` lines 46 to 58

```text
  201 | ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec <
```


### 34 normalised tokens: `crates/widevine/src/pssh.rs` lines 59 to 62, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/mod.rs` lines 24 to 26

```text
   59 | [
   60 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d ,
   61 | 0x21 , 0xed ,
   62 | ]
```


### 34 normalised tokens: `crates/widevine/src/runtime.rs` lines 195 to 207, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 79 to 87

```text
  195 | }
  198 | # [ derive ( Default ) ]
  199 | pub ( crate ) struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64
```


### 34 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1534 to 1538, and kopuz-a58c385 `daemon/player/src/engine/tests.rs` lines 2009 to 2010

```text
 1534 | calls
 1535 | . iter ( )
 1536 | . any ( | call | matches ! ( call , Call :: CloseSession { .. } ) )
 1537 | ) ;
 1538 | assert_eq ! ( calls . last ( )
```


### 34 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 6 to 14, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 5 to 13

```text
    6 | mod fake ;
    7 | mod find ;
    8 | mod host ;
    9 | mod loader ;
   10 | mod round_trip ;
   11 | mod sequence ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync ::
```


### 34 normalised tokens: `crates/widevine/src/tests/sequence.rs` lines 13 to 16, and kopuz-a58c385 `apps/kopuz/components/src/detail_actions.rs` lines 9 to 13

```text
   13 | Rng , key , start } ;
   14 | use crate :: boundary :: { MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST } ;
   15 | use crate :: runtime :: { Deadlines , Runtime } ;
   16 | use crate :: { Error
```


### 34 normalised tokens: `crates/widevine/src/timers.rs` lines 4 to 11, and kopuz-a58c385 `daemon/core/src/offline.rs` lines 8 to 14

```text
    4 | cmp :: Reverse ;
    5 | use std :: collections :: BinaryHeap ;
    6 | use std :: time :: { Duration , Instant } ;
    8 | use crate :: boundary :: TimerContext ;
   11 | # [ derive ( Default
```


### 33 normalised tokens: `crates/widevine/src/boundary.rs` lines 13 to 17, and kopuz-a58c385 `daemon/server/src/ytmusic/player.rs` lines 11 to 16

```text
   13 | ;
   14 | use std :: ffi :: { c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std :: ptr ;
   17 | use std :: time ::
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 42 to 52, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/wayland/window.rs` lines 737 to 746

```text
   42 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
   43 | enum KeyStatus {
   44 | Usable ,
   45 | InternalError ,
   46 | Expired ,
   47 | OutputRestricted ,
   48 | OutputDownscaled ,
   49 | Pending ,
   50 | Released ,
   52 | Unknown
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 201 to 206, and kopuz-a58c385 `daemon/server/src/stream/range_source.rs` lines 31 to 39

```text
  201 | shared : Arc < Shared > ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec <
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 1180 to 1184, and kopuz-a58c385 `daemon/core/src/session/reconciler.rs` lines 175 to 181

```text
 1180 | {
 1181 | self . sessions . iter_mut ( ) . find ( | entry | entry . key == session )
 1182 | }
 1184 | fn session_index ( & self , session : SessionKey ) -> Option
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 1189 to 1192, and kopuz-a58c385 `apps/kopuz/components/src/schema_form.rs` lines 207 to 210

```text
 1189 | ) -> Option < usize > {
 1190 | self . sessions
 1191 | . iter ( )
 1192 | . position ( | entry | entry . cdm_id . as_deref ( ) == Some ( cdm_id ) )
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 26 to 36, and kopuz-a58c385 `daemon/server/src/source/types.rs` lines 178 to 187

```text
   26 | struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
   32 | }
   34 | impl Deadlines {
   36 | pub
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 129 to 138, and kopuz-a58c385 `daemon/server/src/applemusic/progressive.rs` lines 75 to 93

```text
  129 | control : VecDeque < Request > ,
  130 | decrypts : VecDeque < Request > ,
  132 | shutdown_started : Option < Instant > ,
  134 | thread_ended : bool ,
  135 | }
  137 | impl Queue {
  138 | fn push ( &
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 317 to 322, and kopuz-a58c385 `apps/kopuz/library/src/lib.rs` lines 493 to 497

```text
  317 | pub ( crate ) fn shared ( & self ) -> & Shared {
  318 | & self . shared
  319 | }
  322 | pub ( crate ) fn initialise ( & self ) -> Result < (
```


### 33 normalised tokens: `crates/widevine/src/tests/host.rs` lines 11 to 14, and kopuz-a58c385 `daemon/player/src/player.rs` lines 10 to 13

```text
   11 | :: { QUICK , key , ready , start } ;
   12 | use crate :: boundary :: {
   13 | HOST_CALLS , MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
```


### 33 normalised tokens: `crates/widevine/src/timers.rs` lines 14 to 23, and kopuz-a58c385 `apps/kopuz/onboarding/src/lib.rs` lines 38 to 47

```text
   14 | ,
   15 | }
   17 | # [ derive ( Debug , PartialEq , Eq , PartialOrd , Ord ) ]
   18 | struct Timer {
   19 | due : Instant ,
   21 | order : u64 ,
   22 | context : Context ,
   23 | }
```


### 32 normalised tokens: `crates/widevine/examples/probe.rs` lines 44 to 48, and kopuz-a58c385 `daemon/db/src/backend/migrations.rs` lines 1098 to 1102

```text
   44 | now ( ) ;
   45 | let cdm = match widevine :: Cdm :: load ( & path ) {
   46 | Ok ( cdm ) => cdm ,
   47 | Err ( error ) => {
   48 | println
```


### 32 normalised tokens: `crates/widevine/src/find.rs` lines 3 to 7, and kopuz-a58c385 `apps/legacy/pages/src/library.rs` lines 9 to 13

```text
    3 | use std :: cmp :: Ordering ;
    4 | use std :: ffi :: OsStr ;
    5 | use std :: fs ;
    6 | use std :: io :: Read ;
    7 | use std :: os :: unix
```


### 32 normalised tokens: `crates/widevine/src/find.rs` lines 235 to 240, and kopuz-a58c385 `daemon/server/src/applemusic/progressive.rs` lines 832 to 834

```text
  235 | subfolders ( & parent )
  236 | . into_iter ( )
  237 | . filter ( | folder | {
  238 | folder
  239 | . file_name ( )
  240 | . is_some_and ( | name | name . as_bytes ( )
```


### 32 normalised tokens: `crates/widevine/src/host.rs` lines 22 to 25, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 13 to 17

```text
   22 | , DecryptInput , DecryptOutput , Event , HOST_CALLS , Inbox , Subsample , TimerContext ,
   23 | } ;
   24 | use crate :: error :: { decrypt_status_error , exception_name } ;
   25 | use crate :: library
```


### 32 normalised tokens: `crates/widevine/src/host.rs` lines 768 to 773, and kopuz-a58c385 `daemon/player/src/engine/actor.rs` lines 822 to 827

```text
  768 | ) ;
  769 | }
  770 | }
  772 | fn session_named ( & mut self , promise : u32 , cdm_id : Vec < u8 > ) {
  773 | let Some ( index ) = self . wait_for_promise
```


### 32 normalised tokens: `crates/widevine/src/runtime.rs` lines 26 to 31, and kopuz-a58c385 `daemon/server/src/nextcloud/tree.rs` lines 43 to 48

```text
   26 | pub ( crate ) struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
```


### 32 normalised tokens: `crates/widevine/src/runtime.rs` lines 282 to 293, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/headless/window.rs` lines 25 to 31

```text
  282 | # [ derive ( Clone ) ]
  283 | pub ( crate ) struct Runtime {
  284 | shared : Arc < Shared > ,
  285 | }
  287 | impl Runtime {
  293 | pub ( crate ) fn start (
```


### 32 normalised tokens: `crates/widevine/src/tests/cpp_layer.rs` lines 56 to 62, and kopuz-a58c385 `daemon/player/src/eq.rs` lines 8 to 14

```text
   56 | # [ derive ( Debug , PartialEq ) ]
   57 | struct BoundarySizes {
   58 | key_info : u32 ,
   59 | subsample : u32 ,
   60 | decrypt_input : u32 ,
   61 | decrypt_output : u32 ,
   62 | host_calls : u32 ,
```


### 32 normalised tokens: `crates/widevine/src/tests/host.rs` lines 13 to 17, and kopuz-a58c385 `apps/kopuz/components/src/cover.rs` lines 13 to 17

```text
   13 | HOST_CALLS , MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
   15 | } ;
   16 | use crate :: runtime :: { Deadlines , Runtime } ;
   17 | use crate ::
```


### 32 normalised tokens: `crates/widevine/src/tests/host.rs` lines 869 to 875, and kopuz-a58c385 `vendor/gpui-pre-linux/src/linux/platform.rs` lines 1530 to 1536

```text
  869 | ) . unwrap ( ) ;
  870 | assert ! ( runtime . shutdown ( ) ) ;
  871 | }
  873 | # [ test ]
  874 | fn a_session_the_cdm_closes_fails_its_licence_and_decrypts ( ) {
  875 | let fake = Fake :: new
```


### 32 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-a58c385 `daemon/core/src/offline.rs` lines 3 to 6

```text
    5 | use std :: path :: Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync ::
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 11 to 15, and kopuz-a58c385 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   11 | ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 41 to 48, and kopuz-a58c385 `daemon/server/src/cookies/mozilla.rs` lines 85 to 92

```text
   41 | }
   42 | }
   44 | impl Drop for TestFolder {
   45 | fn drop ( & mut self ) {
   46 | let _ = std :: fs :: remove_dir_all ( & self . path ) ;
   47 | }
   48 | }
```


### 31 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 75, and kopuz-a58c385 `daemon/server/shim/widevine_shim.cc` lines 56 to 61

```text
   70 | ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
   74 | private :
   75 | cdm :: Buffer
```


### 31 normalised tokens: `crates/widevine/src/find.rs` lines 66 to 74, and kopuz-a58c385 `apps/kopuz/store/src/layout.rs` lines 759 to 763

```text
   66 | struct Places {
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf
```


### 31 normalised tokens: `crates/widevine/src/find.rs` lines 235 to 240, and kopuz-a58c385 `apps/kopuz/link/src/replace.rs` lines 79 to 83

```text
  235 | )
  236 | . into_iter ( )
  237 | . filter ( | folder | {
  238 | folder
  239 | . file_name ( )
  240 | . is_some_and ( | name | name . as_bytes ( ) . starts_with (
```


### 31 normalised tokens: `crates/widevine/src/host.rs` lines 612 to 619, and kopuz-a58c385 `daemon/core/src/session/mod.rs` lines 1767 to 1775

```text
  612 | }
  614 | fn destroy_instance ( & mut self ) {
  615 | let Some ( instance ) = self . instance . take ( ) else {
  616 | return ;
  617 | } ;
  619 | self . timers
```


### 31 normalised tokens: `crates/widevine/src/manifest.rs` lines 116 to 127, and kopuz-a58c385 `daemon/kopuzd/src/identity.rs` lines 88 to 99

```text
  116 | }
  117 | }
  118 | }
  119 | }
  121 | # [ cfg ( test ) ]
  122 | mod tests {
  123 | use super :: * ;
  125 | # [ test ]
  126 | fn the_version_is_read_from_the_top_level_object ( ) {
  127 | let text =
```


### 31 normalised tokens: `crates/widevine/src/runtime.rs` lines 128 to 134, and kopuz-a58c385 `daemon/player/src/engine/rt.rs` lines 122 to 126

```text
  128 | pub ( crate ) struct Queue {
  129 | control : VecDeque < Request > ,
  130 | decrypts : VecDeque < Request > ,
  132 | shutdown_started : Option < Instant > ,
  134 | thread_ended : bool
```


### 31 normalised tokens: `crates/widevine/src/tests/find.rs` lines 314 to 317, and kopuz-a58c385 `daemon/server/src/lyrics.rs` lines 1007 to 1011

```text
  314 | ;
  315 | let candidates = candidates_in ( & missing ) ;
  316 | assert_eq ! ( candidates . len ( ) , 1 ) ;
  317 | assert ! ( candidates [ 0 ] . set_aside
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1109 to 1113, and kopuz-a58c385 `daemon/db/tests/artist_images.rs` lines 13 to 17

```text
 1109 | ;
 1110 | let now = std :: time :: SystemTime :: now ( )
 1111 | . duration_since ( std :: time :: UNIX_EPOCH )
 1112 | . unwrap ( )
 1113 | . as_secs_f64 ( ) ;
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1110 to 1114, and kopuz-a58c385 `daemon/reader/src/cover_indexer.rs` lines 140 to 144

```text
 1110 | let now = std :: time :: SystemTime :: now ( )
 1111 | . duration_since ( std :: time :: UNIX_EPOCH )
 1112 | . unwrap ( )
 1113 | . as_secs_f64 ( ) ;
 1114 | assert
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1205 to 1208, and kopuz-a58c385 `apps/kopuz/store/src/query.rs` lines 219 to 222

```text
 1205 | # [ test ]
 1206 | fn each_cdm_status_is_its_own_error_and_leaves_the_sample_unchanged ( ) {
 1207 | let status = Arc :: new ( Mutex :: new ( 0 ) ) ;
 1208 | let chosen = Arc :: clone (
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1532 to 1537, and kopuz-a58c385 `daemon/server/src/ytmusic/fixture_tests.rs` lines 192 to 197

```text
 1532 | ) ;
 1533 | assert ! (
 1534 | calls
 1535 | . iter ( )
 1536 | . any ( | call | matches ! ( call , Call :: CloseSession { .. } ) )
 1537 | ) ;
```


### 31 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 13 to 16, and kopuz-a58c385 `daemon/player/src/engine/worker.rs` lines 10 to 13

```text
   13 | Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time :: Duration
```


### 30 normalised tokens: `crates/widevine/src/boundary.rs` lines 209 to 222, and kopuz-a58c385 `apps/legacy/hooks/src/use_player_controller.rs` lines 600 to 613

```text
  209 | HostCalls = HostCalls {
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
```


### 30 normalised tokens: `crates/widevine/src/boundary.rs` lines 240 to 249, and kopuz-a58c385 `daemon/server/src/ytmusic/discover.rs` lines 1346 to 1356

```text
  240 | ,
  241 | NewSessionResolved {
  242 | promise : u32 ,
  243 | session : Vec < u8 > ,
  244 | } ,
  245 | Resolved {
  246 | promise : u32 ,
  247 | } ,
  248 | Rejected {
  249 | promise : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/find.rs` lines 248 to 252, and kopuz-a58c385 `shared/config/src/store.rs` lines 678 to 682

```text
  248 | fs :: read_dir ( folder ) else {
  249 | return Vec :: new ( ) ;
  250 | } ;
  251 | let mut folders : Vec < PathBuf > = entries
  252 | . filter_map (
```


### 30 normalised tokens: `crates/widevine/src/find.rs` lines 252 to 254, and kopuz-a58c385 `apps/legacy/kopuz/src/updates.rs` lines 491 to 493

```text
  252 | . filter_map ( Result :: ok )
  253 | . map ( | entry | entry . path ( ) )
  254 | . filter ( | path | path . is_dir ( )
```


### 30 normalised tokens: `crates/widevine/src/host.rs` lines 207 to 214, and kopuz-a58c385 `apps/kopuz/components/src/fields.rs` lines 415 to 420

```text
  207 | ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/pssh.rs` lines 44 to 54, and kopuz-a58c385 `apps/kopuz/components/src/logo.rs` lines 96 to 106

```text
   44 | ) ;
   45 | pssh
   46 | }
   48 | # [ cfg ( test ) ]
   49 | mod tests {
   50 | use super :: * ;
   52 | # [ test ]
   53 | fn the_pssh_for_a_key_id_is_pinned ( ) {
   54 | let key_id
```


### 30 normalised tokens: `crates/widevine/src/runtime.rs` lines 386 to 389, and kopuz-a58c385 `apps/legacy/components/src/track/list_view.rs` lines 95 to 98

```text
  386 | sample . iv ,
  387 | subsamples : sample . subsamples . as_ptr ( ) ,
  388 | subsample_count : sample . subsamples . len ( ) ,
  389 | scheme : sample . scheme ,
```


### 30 normalised tokens: `crates/widevine/src/tests/cpp_layer.rs` lines 56 to 63, and kopuz-a58c385 `daemon/player/src/systemint/windows.rs` lines 59 to 66

```text
   56 | PartialEq ) ]
   57 | struct BoundarySizes {
   58 | key_info : u32 ,
   59 | subsample : u32 ,
   60 | decrypt_input : u32 ,
   61 | decrypt_output : u32 ,
   62 | host_calls : u32 ,
   63 | cdm_calls : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 83 to 90, and kopuz-a58c385 `daemon/server/src/ytmusic/discover.rs` lines 172 to 180

```text
   83 | u32 ) ,
   84 | Message {
   85 | session : Vec < u8 > ,
   86 | kind : u32 ,
   87 | message : Vec < u8 > ,
   88 | } ,
   89 | Keys {
   90 | session : Vec
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 70 to 75, and kopuz-a58c385 `daemon/core/src/session/tests.rs` lines 5000 to 5006

```text
   70 | }
   72 | fn decrypts ( fake : & Fake ) -> Vec < DecryptCall > {
   73 | fake . calls ( )
   74 | . into_iter ( )
   75 | . filter_map ( | call |
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 200 to 205, and kopuz-a58c385 `daemon/core/src/session/tests.rs` lines 5000 to 5006

```text
  200 | }
  202 | fn storage_answers ( fake : & Fake ) -> Vec < Call > {
  203 | fake . calls ( )
  204 | . into_iter ( )
  205 | . filter ( | call |
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 295 to 301, and kopuz-a58c385 `daemon/server/src/applemusic/widevine/fetch.rs` lines 591 to 597

```text
  295 | ) ;
  296 | assert ! ( storage_answers ( & fake ) . is_empty ( ) ) ;
  297 | }
  299 | # [ test ]
  300 | fn host_calls_of_a_creation_that_gives_no_instance_are_dropped ( ) {
  301 | let mut creates =
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 326 to 332, and kopuz-a58c385 `shared/config/src/store.rs` lines 1217 to 1223

```text
  326 | ) ;
  327 | assert ! ( storage_answers ( & fake ) . is_empty ( ) ) ;
  328 | }
  330 | # [ test ]
  331 | fn an_instance_of_another_interface_is_refused ( ) {
  332 | let fake = Fake
```


### 30 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-a58c385 `daemon/core/src/auto_sync.rs` lines 257 to 260

```text
    5 | path :: Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync :: mpsc
```


### 30 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-a58c385 `apps/kopuz/link/src/replace.rs` lines 97 to 101

```text
    5 | Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync :: mpsc :: channel
```


### 30 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 13 to 18, and kopuz-a58c385 `daemon/player/src/engine/mod.rs` lines 20 to 26

```text
   13 | } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time :: Duration ;
   18 | use
```


### 119 normalised tokens: `crates/widevine/src/pssh.rs` lines 36 to 53, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/mod.rs` lines 463 to 482

```text
   36 | + data . len ( ) ;
   37 | let mut pssh = Vec :: with_capacity ( size ) ;
   39 | pssh . extend_from_slice ( & ( size as u32 ) . to_be_bytes ( ) ) ;
   40 | pssh . extend_from_slice ( b"pssh" ) ;
   41 | pssh . extend_from_slice ( & [ 0 , 0 , 0 , 0 ] ) ;
   42 | pssh . extend_from_slice ( & SYSTEM_ID ) ;
   43 | pssh . extend_from_slice ( & ( data . len ( ) as u32 ) . to_be_bytes ( ) ) ;
   44 | pssh . extend_from_slice ( & data ) ;
   45 | pssh
   46 | }
   48 | # [ cfg ( test ) ]
   49 | mod tests {
   50 | use super :: * ;
   52 | # [ test ]
   53 | fn the_pssh_for_a_key_id_is_pinned ( ) {
```


### 56 normalised tokens: `crates/widevine/src/tests/host.rs` lines 134 to 141, and kopuz-1d8b323 `daemon/server/src/ytmusic/fixture_tests.rs` lines 768 to 775

```text
  134 | ) ;
  135 | let creates = calls
  136 | . iter ( )
  137 | . filter ( | call | matches ! ( call , Call :: Create { .. } ) )
  138 | . count ( ) ;
  139 | let destroys = calls
  140 | . iter ( )
  141 | . filter ( | call | matches ! ( call , Call :: Destroy
```


### 55 normalised tokens: `crates/widevine/src/host.rs` lines 222 to 230, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 828 to 836

```text
  222 | ,
  223 | inbox : Inbox :: for_this_thread ( ) ,
  224 | instance : None ,
  225 | timers : Timers :: default ( ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise :
```


### 49 normalised tokens: `crates/widevine/src/tests/find.rs` lines 82 to 88, and kopuz-1d8b323 `daemon/db/src/backend/queries.rs` lines 1190 to 1196

```text
   82 | }
   84 | fn find < 'a > ( candidates : & 'a [ Candidate ] , path : & Path ) -> & 'a Candidate {
   85 | candidates
   86 | . iter ( )
   87 | . find ( | candidate | candidate . path == path )
   88 | . unwrap_or_else ( || panic ! (
```


### 46 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 215, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 378 to 389

```text
  199 | struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
  215 | impl Shared
```


### 45 normalised tokens: `crates/widevine/src/pssh.rs` lines 4 to 6, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/mod.rs` lines 24 to 26

```text
    4 | pub const SYSTEM_ID : [ u8 ; 16 ] = [
    5 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d , 0x21 , 0xed ,
    6 | ] ;
```


### 45 normalised tokens: `crates/widevine/src/tests/host.rs` lines 13 to 21, and kopuz-1d8b323 `apps/kopuz/components/src/carousel.rs` lines 9 to 17

```text
   13 | MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
   15 | } ;
   16 | use crate :: runtime :: { Deadlines , Runtime } ;
   17 | use crate :: { Error , Protected , Scheme , pssh_for_key } ;
   21 | const HANG_GUARD : Duration =
```


### 45 normalised tokens: `crates/widevine/src/tests/sequence.rs` lines 13 to 18, and kopuz-1d8b323 `apps/kopuz/components/src/track_meta.rs` lines 16 to 21

```text
   13 | key , start } ;
   14 | use crate :: boundary :: { MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST } ;
   15 | use crate :: runtime :: { Deadlines , Runtime } ;
   16 | use crate :: { Error , Protected , Scheme , pssh_for_key } ;
   18 | const SEQUENCES : u64 =
```


### 43 normalised tokens: `crates/widevine/src/boundary.rs` lines 210 to 226, and kopuz-1d8b323 `shared/config/src/ui_prefs.rs` lines 8 to 14

```text
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
  223 | } ;
  226 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
```


### 43 normalised tokens: `crates/widevine/src/host.rs` lines 202 to 208, and kopuz-1d8b323 `apps/kopuz/components/src/steps.rs` lines 303 to 309

```text
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
```


### 43 normalised tokens: `crates/widevine/src/host.rs` lines 203 to 210, and kopuz-1d8b323 `apps/kopuz/components/src/folders.rs` lines 176 to 183

```text
  203 | ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32
```


### 42 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 405 to 416, and kopuz-1d8b323 `apps/legacy/kopuz/src/main.rs` lines 1536 to 1547

```text
  405 | ,
  406 | create : fake_create ,
  407 | initialize : fake_initialize ,
  408 | create_session : fake_create_session ,
  409 | update_session : fake_update_session ,
  410 | close_session : fake_close_session ,
  411 | timer_expired : fake_timer_expired ,
  412 | decrypt : fake_decrypt ,
  413 | release_buffer : fake_release_buffer ,
  414 | storage_id : fake_storage_id ,
  415 | destroy : fake_destroy ,
  416 | }
```


### 41 normalised tokens: `crates/widevine/src/find.rs` lines 139 to 143, and kopuz-1d8b323 `apps/legacy/hooks/src/use_player_controller.rs` lines 173 to 177

```text
  139 | fn choose ( candidates : & [ Candidate ] ) -> Option < PathBuf > {
  140 | candidates
  141 | . iter ( )
  142 | . filter ( | candidate | candidate . set_aside . is_none ( ) )
  143 | . map ( | candidate |
```


### 41 normalised tokens: `crates/widevine/src/lib.rs` lines 52 to 59, and kopuz-1d8b323 `shared/api/src/lib.rs` lines 80 to 86

```text
   52 | find :: { cdm_candidates , find_cdm } ;
   53 | pub use pssh :: { SYSTEM_ID , pssh_for_key } ;
   56 | # [ derive ( Debug , Clone , PartialEq , Eq ) ]
   57 | pub struct Candidate {
   59 | pub path : std :: path
```


### 40 normalised tokens: `crates/widevine/src/host.rs` lines 204 to 212, and kopuz-1d8b323 `apps/kopuz/palette/src/lib.rs` lines 143 to 153

```text
  204 | ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec
```


### 40 normalised tokens: `crates/widevine/src/host.rs` lines 225 to 231, and kopuz-1d8b323 `daemon/discord-presence/src/cover_art.rs` lines 295 to 301

```text
  225 | ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise : 0 ,
  231 | subsamples :
```


### 40 normalised tokens: `crates/widevine/src/lib.rs` lines 52 to 59, and kopuz-1d8b323 `apps/kopuz/theme/src/lib.rs` lines 19 to 25

```text
   52 | pub use find :: { cdm_candidates , find_cdm } ;
   53 | pub use pssh :: { SYSTEM_ID , pssh_for_key } ;
   56 | # [ derive ( Debug , Clone , PartialEq , Eq ) ]
   57 | pub struct Candidate {
   59 | pub path :
```


### 40 normalised tokens: `crates/widevine/src/runtime.rs` lines 203 to 216, and kopuz-1d8b323 `apps/kopuz/components/src/transport.rs` lines 38 to 49

```text
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
  215 | impl Shared {
  216 | pub
```


### 40 normalised tokens: `crates/widevine/src/tests/host.rs` lines 4 to 8, and kopuz-1d8b323 `daemon/core/src/offline.rs` lines 4 to 9

```text
    4 | sync :: atomic :: Ordering ;
    5 | use std :: sync :: mpsc :: { Receiver , channel } ;
    6 | use std :: sync :: { Arc , Mutex } ;
    7 | use std :: thread ;
    8 | use std :: time ::
```


### 40 normalised tokens: `crates/widevine/src/timers.rs` lines 149 to 156, and kopuz-1d8b323 `apps/kopuz/primitives/src/motion.rs` lines 414 to 421

```text
  149 | ) ) ;
  150 | }
  152 | # [ test ]
  153 | fn negative_and_huge_delays ( ) {
  154 | let start = Instant :: now ( ) ;
  155 | let mut timers = Timers :: default ( ) ;
  156 | assert ! ( timers . set ( start
```


### 38 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 49 to 52, and kopuz-1d8b323 `daemon/server/shim/widevine_shim.cc` lines 268 to 271

```text
   49 | capacity_ ; }
   50 | uint8_t * Data ( ) override { return data_ ; }
   51 | void SetSize ( uint32_t size ) override { size_ = size ; }
   52 | uint32_t Size ( ) const override { return size_ ; }
```


### 38 normalised tokens: `crates/widevine/src/desktop.rs` lines 118 to 125, and kopuz-1d8b323 `apps/kopuz/store/src/target.rs` lines 65 to 71

```text
  118 | key ) ;
  119 | }
  120 | }
  122 | impl fmt :: Debug for Session {
  123 | fn fmt ( & self , formatter : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
  124 | formatter
  125 | . debug_struct (
```


### 38 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 36 to 45, and kopuz-1d8b323 `daemon/server/src/ytmusic/discover.rs` lines 1338 to 1347

```text
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec < u8 > ,
   39 | response : Vec < u8 > ,
   40 | } ,
   41 | CloseSession {
   42 | promise : u32 ,
   43 | session : Vec < u8 > ,
   44 | } ,
   45 | Decrypt
```


### 38 normalised tokens: `crates/widevine/src/tests/host.rs` lines 953 to 959, and kopuz-1d8b323 `daemon/server/src/ytmusic/fixture_tests.rs` lines 562 to 568

```text
  953 | )
  954 | . filter_map ( | call | match call {
  955 | Call :: CloseSession { session , .. } => Some ( session ) ,
  956 | _ => None ,
  957 | } )
  958 | . collect ( ) ;
  959 | assert_eq ! (
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 196 to 205, and kopuz-1d8b323 `apps/kopuz/shortcuts/src/editor.rs` lines 18 to 26

```text
  196 | ,
  197 | }
  200 | pub ( crate ) struct Host {
  201 | shared : Arc < Shared > ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 203 to 209, and kopuz-1d8b323 `apps/kopuz/page_albums/src/detail.rs` lines 50 to 57

```text
  203 | > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 205 to 212, and kopuz-1d8b323 `apps/kopuz/metadata_dialog/src/lib.rs` lines 74 to 80

```text
  205 | ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec <
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 1176 to 1178, and kopuz-1d8b323 `apps/kopuz/store/src/jobs.rs` lines 202 to 204

```text
 1176 | fn session ( & self , session : SessionKey ) -> Option < & SessionEntry > {
 1177 | self . sessions . iter ( ) . find ( | entry | entry . key == session )
 1178 | }
```


### 37 normalised tokens: `crates/widevine/src/host.rs` lines 1177 to 1181, and kopuz-1d8b323 `apps/kopuz/discover/src/shelves.rs` lines 164 to 170

```text
 1177 | )
 1178 | }
 1180 | fn session_mut ( & mut self , session : SessionKey ) -> Option < & mut SessionEntry > {
 1181 | self . sessions . iter_mut ( ) . find ( | entry | entry . key
```


### 37 normalised tokens: `crates/widevine/src/lib.rs` lines 59 to 68, and kopuz-1d8b323 `shared/api/src/sources.rs` lines 272 to 279

```text
   59 | PathBuf ,
   61 | pub version : Option < String > ,
   63 | pub set_aside : Option < String > ,
   64 | }
   67 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
   68 | pub struct Protected
```


### 37 normalised tokens: `crates/widevine/src/timers.rs` lines 116 to 122, and kopuz-1d8b323 `apps/kopuz/primitives/src/motion.rs` lines 415 to 421

```text
  116 | }
  118 | # [ test ]
  119 | fn timers_fire_in_order_of_due_time_and_never_before ( ) {
  120 | let start = Instant :: now ( ) ;
  121 | let mut timers = Timers :: default ( ) ;
  122 | assert ! ( timers . set ( start
```


### 36 normalised tokens: `crates/widevine/src/boundary.rs` lines 13 to 16, and kopuz-1d8b323 `daemon/core/src/config_service.rs` lines 3 to 7

```text
   13 | use std :: cell :: { Cell , RefCell } ;
   14 | use std :: ffi :: { c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std ::
```


### 36 normalised tokens: `crates/widevine/src/desktop.rs` lines 145 to 155, and kopuz-1d8b323 `daemon/core/src/spotify.rs` lines 39 to 46

```text
  145 | loading : Mutex < Loaded > ,
  147 | current : Mutex < Current > ,
  148 | }
  150 | # [ derive ( Default ) ]
  151 | struct Loaded {
  153 | path : Option < PathBuf > ,
  155 | ready : bool ,
```


### 36 normalised tokens: `crates/widevine/src/find.rs` lines 187 to 193, and kopuz-1d8b323 `daemon/core/src/url_download.rs` lines 777 to 782

```text
  187 | )
  188 | . find ( | ordering | ordering . is_ne ( ) )
  189 | . unwrap_or ( Ordering :: Equal )
  190 | }
  193 | fn chromium_trees ( config : & Path , platform : & str , found :
```


### 36 normalised tokens: `crates/widevine/src/host.rs` lines 206 to 214, and kopuz-1d8b323 `apps/kopuz/components/src/source_card.rs` lines 20 to 26

```text
  206 | ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32
```


### 36 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 209, and kopuz-1d8b323 `daemon/server/src/nextcloud/tree.rs` lines 43 to 49

```text
  199 | pub ( crate ) struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64
```


### 36 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 9 to 12, and kopuz-1d8b323 `apps/kopuz/components/src/settings.rs` lines 6 to 9

```text
    9 | c_char , c_void } ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicI64 , Ordering } ;
   11 | use std :: sync :: mpsc :: Sender ;
   12 | use std :: sync :: { Arc
```


### 36 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 31 to 35, and kopuz-1d8b323 `daemon/server/src/applemusic/stream.rs` lines 884 to 886

```text
   31 | ,
   32 | std :: process :: id ( )
   33 | ) ) ;
   34 | let _ = std :: fs :: remove_dir_all ( & path ) ;
   35 | std :: fs :: create_dir_all ( & path ) . expect (
```


### 36 normalised tokens: `crates/widevine/src/timers.rs` lines 137 to 144, and kopuz-1d8b323 `apps/kopuz/primitives/src/motion.rs` lines 428 to 435

```text
  137 | ) ) ;
  138 | }
  140 | # [ test ]
  141 | fn a_timer_set_during_a_round_waits_for_the_next ( ) {
  142 | let start = Instant :: now ( ) ;
  143 | let mut timers = Timers :: default ( ) ;
  144 | timers . set (
```


### 35 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 72, and kopuz-1d8b323 `daemon/server/shim/widevine_shim.cc` lines 269 to 271

```text
   70 | Buffer * DecryptedBuffer ( ) override { return buffer_ ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
```


### 35 normalised tokens: `crates/widevine/src/host.rs` lines 226 to 231, and kopuz-1d8b323 `daemon/core/src/artwork.rs` lines 643 to 648

```text
  226 | new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise : 0 ,
  231 | subsamples : Vec
```


### 35 normalised tokens: `crates/widevine/src/pssh.rs` lines 58 to 61, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/mod.rs` lines 509 to 512

```text
   58 | ] ,
   59 | & [
   60 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d ,
   61 | 0x21 , 0xed
```


### 35 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 28 to 38, and kopuz-1d8b323 `daemon/server/src/catalog.rs` lines 31 to 41

```text
   28 | ,
   29 | } ,
   30 | CreateSession {
   31 | promise : u32 ,
   32 | session_type : u32 ,
   33 | init_data_type : u32 ,
   34 | init_data : Vec < u8 > ,
   35 | } ,
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec
```


### 35 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 57 to 62, and kopuz-1d8b323 `daemon/server/src/ytmusic/search.rs` lines 41 to 46

```text
   57 | ) struct DecryptCall {
   58 | pub size : usize ,
   59 | pub scheme : u32 ,
   60 | pub key_id : Vec < u8 > ,
   61 | pub iv : Vec < u8 > ,
   62 | pub subsamples : Vec <
```


### 34 normalised tokens: `crates/widevine/src/boundary.rs` lines 209 to 226, and kopuz-1d8b323 `daemon/player/src/player.rs` lines 12 to 16

```text
  209 | {
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
  223 | } ;
  226 | # [ derive ( Debug
```


### 34 normalised tokens: `crates/widevine/src/find.rs` lines 68 to 76, and kopuz-1d8b323 `daemon/db/src/backend/rows.rs` lines 26 to 30

```text
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf ,
   76 | pub platform : Option <
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 11 to 16, and kopuz-1d8b323 `apps/kopuz/onboarding/src/ui.rs` lines 4 to 9

```text
   11 | use std :: collections :: VecDeque ;
   12 | use std :: ffi :: c_void ;
   13 | use std :: fmt ;
   14 | use std :: ptr ;
   15 | use std :: sync :: Arc ;
   16 | use std ::
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 201 to 207, and kopuz-1d8b323 `daemon/core/src/spotify.rs` lines 46 to 58

```text
  201 | ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec <
```


### 34 normalised tokens: `crates/widevine/src/pssh.rs` lines 59 to 62, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/mod.rs` lines 24 to 26

```text
   59 | [
   60 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d ,
   61 | 0x21 , 0xed ,
   62 | ]
```


### 34 normalised tokens: `crates/widevine/src/runtime.rs` lines 195 to 207, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 79 to 87

```text
  195 | }
  198 | # [ derive ( Default ) ]
  199 | pub ( crate ) struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64
```


### 34 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1534 to 1538, and kopuz-1d8b323 `daemon/player/src/engine/tests.rs` lines 2009 to 2010

```text
 1534 | calls
 1535 | . iter ( )
 1536 | . any ( | call | matches ! ( call , Call :: CloseSession { .. } ) )
 1537 | ) ;
 1538 | assert_eq ! ( calls . last ( )
```


### 34 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 6 to 14, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 5 to 13

```text
    6 | mod fake ;
    7 | mod find ;
    8 | mod host ;
    9 | mod loader ;
   10 | mod round_trip ;
   11 | mod sequence ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync ::
```


### 34 normalised tokens: `crates/widevine/src/tests/sequence.rs` lines 13 to 16, and kopuz-1d8b323 `apps/kopuz/components/src/detail_actions.rs` lines 9 to 13

```text
   13 | Rng , key , start } ;
   14 | use crate :: boundary :: { MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST } ;
   15 | use crate :: runtime :: { Deadlines , Runtime } ;
   16 | use crate :: { Error
```


### 34 normalised tokens: `crates/widevine/src/timers.rs` lines 4 to 11, and kopuz-1d8b323 `daemon/core/src/offline.rs` lines 8 to 14

```text
    4 | cmp :: Reverse ;
    5 | use std :: collections :: BinaryHeap ;
    6 | use std :: time :: { Duration , Instant } ;
    8 | use crate :: boundary :: TimerContext ;
   11 | # [ derive ( Default
```


### 33 normalised tokens: `crates/widevine/src/boundary.rs` lines 13 to 17, and kopuz-1d8b323 `daemon/server/src/ytmusic/player.rs` lines 11 to 16

```text
   13 | ;
   14 | use std :: ffi :: { c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std :: ptr ;
   17 | use std :: time ::
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 42 to 52, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/wayland/window.rs` lines 737 to 746

```text
   42 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
   43 | enum KeyStatus {
   44 | Usable ,
   45 | InternalError ,
   46 | Expired ,
   47 | OutputRestricted ,
   48 | OutputDownscaled ,
   49 | Pending ,
   50 | Released ,
   52 | Unknown
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 201 to 206, and kopuz-1d8b323 `daemon/server/src/stream/range_source.rs` lines 31 to 39

```text
  201 | shared : Arc < Shared > ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec <
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 1180 to 1184, and kopuz-1d8b323 `daemon/core/src/session/reconciler.rs` lines 175 to 181

```text
 1180 | {
 1181 | self . sessions . iter_mut ( ) . find ( | entry | entry . key == session )
 1182 | }
 1184 | fn session_index ( & self , session : SessionKey ) -> Option
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 1189 to 1192, and kopuz-1d8b323 `apps/kopuz/components/src/schema_form.rs` lines 207 to 210

```text
 1189 | ) -> Option < usize > {
 1190 | self . sessions
 1191 | . iter ( )
 1192 | . position ( | entry | entry . cdm_id . as_deref ( ) == Some ( cdm_id ) )
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 26 to 36, and kopuz-1d8b323 `daemon/server/src/source/types.rs` lines 178 to 187

```text
   26 | struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
   32 | }
   34 | impl Deadlines {
   36 | pub
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 129 to 138, and kopuz-1d8b323 `daemon/server/src/applemusic/progressive.rs` lines 75 to 93

```text
  129 | control : VecDeque < Request > ,
  130 | decrypts : VecDeque < Request > ,
  132 | shutdown_started : Option < Instant > ,
  134 | thread_ended : bool ,
  135 | }
  137 | impl Queue {
  138 | fn push ( &
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 317 to 322, and kopuz-1d8b323 `apps/kopuz/library/src/lib.rs` lines 494 to 498

```text
  317 | pub ( crate ) fn shared ( & self ) -> & Shared {
  318 | & self . shared
  319 | }
  322 | pub ( crate ) fn initialise ( & self ) -> Result < (
```


### 33 normalised tokens: `crates/widevine/src/tests/host.rs` lines 11 to 14, and kopuz-1d8b323 `daemon/player/src/player.rs` lines 10 to 13

```text
   11 | :: { QUICK , key , ready , start } ;
   12 | use crate :: boundary :: {
   13 | HOST_CALLS , MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
```


### 33 normalised tokens: `crates/widevine/src/timers.rs` lines 14 to 23, and kopuz-1d8b323 `apps/kopuz/onboarding/src/lib.rs` lines 38 to 47

```text
   14 | ,
   15 | }
   17 | # [ derive ( Debug , PartialEq , Eq , PartialOrd , Ord ) ]
   18 | struct Timer {
   19 | due : Instant ,
   21 | order : u64 ,
   22 | context : Context ,
   23 | }
```


### 32 normalised tokens: `crates/widevine/examples/probe.rs` lines 44 to 48, and kopuz-1d8b323 `daemon/db/src/backend/migrations.rs` lines 1098 to 1102

```text
   44 | now ( ) ;
   45 | let cdm = match widevine :: Cdm :: load ( & path ) {
   46 | Ok ( cdm ) => cdm ,
   47 | Err ( error ) => {
   48 | println
```


### 32 normalised tokens: `crates/widevine/src/find.rs` lines 3 to 7, and kopuz-1d8b323 `apps/legacy/pages/src/library.rs` lines 9 to 13

```text
    3 | use std :: cmp :: Ordering ;
    4 | use std :: ffi :: OsStr ;
    5 | use std :: fs ;
    6 | use std :: io :: Read ;
    7 | use std :: os :: unix
```


### 32 normalised tokens: `crates/widevine/src/find.rs` lines 235 to 240, and kopuz-1d8b323 `daemon/server/src/applemusic/progressive.rs` lines 832 to 834

```text
  235 | subfolders ( & parent )
  236 | . into_iter ( )
  237 | . filter ( | folder | {
  238 | folder
  239 | . file_name ( )
  240 | . is_some_and ( | name | name . as_bytes ( )
```


### 32 normalised tokens: `crates/widevine/src/host.rs` lines 22 to 25, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 13 to 17

```text
   22 | , DecryptInput , DecryptOutput , Event , HOST_CALLS , Inbox , Subsample , TimerContext ,
   23 | } ;
   24 | use crate :: error :: { decrypt_status_error , exception_name } ;
   25 | use crate :: library
```


### 32 normalised tokens: `crates/widevine/src/host.rs` lines 768 to 773, and kopuz-1d8b323 `daemon/player/src/engine/actor.rs` lines 822 to 827

```text
  768 | ) ;
  769 | }
  770 | }
  772 | fn session_named ( & mut self , promise : u32 , cdm_id : Vec < u8 > ) {
  773 | let Some ( index ) = self . wait_for_promise
```


### 32 normalised tokens: `crates/widevine/src/runtime.rs` lines 26 to 31, and kopuz-1d8b323 `daemon/server/src/nextcloud/tree.rs` lines 43 to 48

```text
   26 | pub ( crate ) struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
```


### 32 normalised tokens: `crates/widevine/src/runtime.rs` lines 282 to 293, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/headless/window.rs` lines 25 to 31

```text
  282 | # [ derive ( Clone ) ]
  283 | pub ( crate ) struct Runtime {
  284 | shared : Arc < Shared > ,
  285 | }
  287 | impl Runtime {
  293 | pub ( crate ) fn start (
```


### 32 normalised tokens: `crates/widevine/src/tests/cpp_layer.rs` lines 56 to 62, and kopuz-1d8b323 `daemon/player/src/eq.rs` lines 8 to 14

```text
   56 | # [ derive ( Debug , PartialEq ) ]
   57 | struct BoundarySizes {
   58 | key_info : u32 ,
   59 | subsample : u32 ,
   60 | decrypt_input : u32 ,
   61 | decrypt_output : u32 ,
   62 | host_calls : u32 ,
```


### 32 normalised tokens: `crates/widevine/src/tests/host.rs` lines 13 to 17, and kopuz-1d8b323 `apps/kopuz/components/src/cover.rs` lines 13 to 17

```text
   13 | HOST_CALLS , MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST , SCHEME_CBCS ,
   14 | SCHEME_CENC , UNANSWERED_FILE_IO , UNANSWERED_METRICS , UNANSWERED_QUERY_OUTPUT_PROTECTION ,
   15 | } ;
   16 | use crate :: runtime :: { Deadlines , Runtime } ;
   17 | use crate ::
```


### 32 normalised tokens: `crates/widevine/src/tests/host.rs` lines 869 to 875, and kopuz-1d8b323 `vendor/gpui-pre-linux/src/linux/platform.rs` lines 1530 to 1536

```text
  869 | ) . unwrap ( ) ;
  870 | assert ! ( runtime . shutdown ( ) ) ;
  871 | }
  873 | # [ test ]
  874 | fn a_session_the_cdm_closes_fails_its_licence_and_decrypts ( ) {
  875 | let fake = Fake :: new
```


### 32 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-1d8b323 `daemon/core/src/offline.rs` lines 3 to 6

```text
    5 | use std :: path :: Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync ::
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 11 to 15, and kopuz-1d8b323 `daemon/reader/src/scanner.rs` lines 7 to 10

```text
   11 | ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 41 to 48, and kopuz-1d8b323 `daemon/server/src/cookies/mozilla.rs` lines 85 to 92

```text
   41 | }
   42 | }
   44 | impl Drop for TestFolder {
   45 | fn drop ( & mut self ) {
   46 | let _ = std :: fs :: remove_dir_all ( & self . path ) ;
   47 | }
   48 | }
```


### 31 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 75, and kopuz-1d8b323 `daemon/server/shim/widevine_shim.cc` lines 56 to 61

```text
   70 | ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
   74 | private :
   75 | cdm :: Buffer
```


### 31 normalised tokens: `crates/widevine/src/find.rs` lines 66 to 74, and kopuz-1d8b323 `apps/kopuz/store/src/layout.rs` lines 784 to 788

```text
   66 | struct Places {
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf
```


### 31 normalised tokens: `crates/widevine/src/find.rs` lines 235 to 240, and kopuz-1d8b323 `apps/kopuz/link/src/replace.rs` lines 79 to 83

```text
  235 | )
  236 | . into_iter ( )
  237 | . filter ( | folder | {
  238 | folder
  239 | . file_name ( )
  240 | . is_some_and ( | name | name . as_bytes ( ) . starts_with (
```


### 31 normalised tokens: `crates/widevine/src/host.rs` lines 612 to 619, and kopuz-1d8b323 `daemon/core/src/session/mod.rs` lines 1791 to 1799

```text
  612 | }
  614 | fn destroy_instance ( & mut self ) {
  615 | let Some ( instance ) = self . instance . take ( ) else {
  616 | return ;
  617 | } ;
  619 | self . timers
```


### 31 normalised tokens: `crates/widevine/src/manifest.rs` lines 116 to 127, and kopuz-1d8b323 `daemon/kopuzd/src/identity.rs` lines 88 to 99

```text
  116 | }
  117 | }
  118 | }
  119 | }
  121 | # [ cfg ( test ) ]
  122 | mod tests {
  123 | use super :: * ;
  125 | # [ test ]
  126 | fn the_version_is_read_from_the_top_level_object ( ) {
  127 | let text =
```


### 31 normalised tokens: `crates/widevine/src/runtime.rs` lines 128 to 134, and kopuz-1d8b323 `daemon/player/src/engine/rt.rs` lines 122 to 126

```text
  128 | pub ( crate ) struct Queue {
  129 | control : VecDeque < Request > ,
  130 | decrypts : VecDeque < Request > ,
  132 | shutdown_started : Option < Instant > ,
  134 | thread_ended : bool
```


### 31 normalised tokens: `crates/widevine/src/tests/find.rs` lines 314 to 317, and kopuz-1d8b323 `daemon/server/src/lyrics.rs` lines 1007 to 1011

```text
  314 | ;
  315 | let candidates = candidates_in ( & missing ) ;
  316 | assert_eq ! ( candidates . len ( ) , 1 ) ;
  317 | assert ! ( candidates [ 0 ] . set_aside
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1109 to 1113, and kopuz-1d8b323 `daemon/db/tests/artist_images.rs` lines 13 to 17

```text
 1109 | ;
 1110 | let now = std :: time :: SystemTime :: now ( )
 1111 | . duration_since ( std :: time :: UNIX_EPOCH )
 1112 | . unwrap ( )
 1113 | . as_secs_f64 ( ) ;
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1110 to 1114, and kopuz-1d8b323 `daemon/reader/src/cover_indexer.rs` lines 140 to 144

```text
 1110 | let now = std :: time :: SystemTime :: now ( )
 1111 | . duration_since ( std :: time :: UNIX_EPOCH )
 1112 | . unwrap ( )
 1113 | . as_secs_f64 ( ) ;
 1114 | assert
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1205 to 1208, and kopuz-1d8b323 `apps/kopuz/store/src/query.rs` lines 219 to 222

```text
 1205 | # [ test ]
 1206 | fn each_cdm_status_is_its_own_error_and_leaves_the_sample_unchanged ( ) {
 1207 | let status = Arc :: new ( Mutex :: new ( 0 ) ) ;
 1208 | let chosen = Arc :: clone (
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 1532 to 1537, and kopuz-1d8b323 `daemon/server/src/ytmusic/fixture_tests.rs` lines 192 to 197

```text
 1532 | ) ;
 1533 | assert ! (
 1534 | calls
 1535 | . iter ( )
 1536 | . any ( | call | matches ! ( call , Call :: CloseSession { .. } ) )
 1537 | ) ;
```


### 31 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 13 to 16, and kopuz-1d8b323 `daemon/player/src/engine/worker.rs` lines 10 to 13

```text
   13 | Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time :: Duration
```


### 30 normalised tokens: `crates/widevine/src/boundary.rs` lines 209 to 222, and kopuz-1d8b323 `apps/legacy/hooks/src/use_player_controller.rs` lines 600 to 613

```text
  209 | HostCalls = HostCalls {
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
```


### 30 normalised tokens: `crates/widevine/src/boundary.rs` lines 240 to 249, and kopuz-1d8b323 `daemon/server/src/ytmusic/discover.rs` lines 1346 to 1356

```text
  240 | ,
  241 | NewSessionResolved {
  242 | promise : u32 ,
  243 | session : Vec < u8 > ,
  244 | } ,
  245 | Resolved {
  246 | promise : u32 ,
  247 | } ,
  248 | Rejected {
  249 | promise : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/find.rs` lines 248 to 252, and kopuz-1d8b323 `shared/config/src/store.rs` lines 678 to 682

```text
  248 | fs :: read_dir ( folder ) else {
  249 | return Vec :: new ( ) ;
  250 | } ;
  251 | let mut folders : Vec < PathBuf > = entries
  252 | . filter_map (
```


### 30 normalised tokens: `crates/widevine/src/find.rs` lines 252 to 254, and kopuz-1d8b323 `apps/legacy/kopuz/src/updates.rs` lines 491 to 493

```text
  252 | . filter_map ( Result :: ok )
  253 | . map ( | entry | entry . path ( ) )
  254 | . filter ( | path | path . is_dir ( )
```


### 30 normalised tokens: `crates/widevine/src/host.rs` lines 207 to 214, and kopuz-1d8b323 `apps/kopuz/components/src/fields.rs` lines 415 to 420

```text
  207 | ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/pssh.rs` lines 44 to 54, and kopuz-1d8b323 `apps/kopuz/components/src/logo.rs` lines 96 to 106

```text
   44 | ) ;
   45 | pssh
   46 | }
   48 | # [ cfg ( test ) ]
   49 | mod tests {
   50 | use super :: * ;
   52 | # [ test ]
   53 | fn the_pssh_for_a_key_id_is_pinned ( ) {
   54 | let key_id
```


### 30 normalised tokens: `crates/widevine/src/runtime.rs` lines 386 to 389, and kopuz-1d8b323 `apps/legacy/components/src/track/list_view.rs` lines 95 to 98

```text
  386 | sample . iv ,
  387 | subsamples : sample . subsamples . as_ptr ( ) ,
  388 | subsample_count : sample . subsamples . len ( ) ,
  389 | scheme : sample . scheme ,
```


### 30 normalised tokens: `crates/widevine/src/tests/cpp_layer.rs` lines 56 to 63, and kopuz-1d8b323 `daemon/player/src/systemint/windows.rs` lines 59 to 66

```text
   56 | PartialEq ) ]
   57 | struct BoundarySizes {
   58 | key_info : u32 ,
   59 | subsample : u32 ,
   60 | decrypt_input : u32 ,
   61 | decrypt_output : u32 ,
   62 | host_calls : u32 ,
   63 | cdm_calls : u32 ,
```


### 30 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 83 to 90, and kopuz-1d8b323 `daemon/server/src/ytmusic/discover.rs` lines 172 to 180

```text
   83 | u32 ) ,
   84 | Message {
   85 | session : Vec < u8 > ,
   86 | kind : u32 ,
   87 | message : Vec < u8 > ,
   88 | } ,
   89 | Keys {
   90 | session : Vec
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 70 to 75, and kopuz-1d8b323 `daemon/core/src/session/tests.rs` lines 5031 to 5037

```text
   70 | }
   72 | fn decrypts ( fake : & Fake ) -> Vec < DecryptCall > {
   73 | fake . calls ( )
   74 | . into_iter ( )
   75 | . filter_map ( | call |
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 200 to 205, and kopuz-1d8b323 `daemon/core/src/session/tests.rs` lines 5031 to 5037

```text
  200 | }
  202 | fn storage_answers ( fake : & Fake ) -> Vec < Call > {
  203 | fake . calls ( )
  204 | . into_iter ( )
  205 | . filter ( | call |
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 295 to 301, and kopuz-1d8b323 `daemon/server/src/applemusic/widevine/fetch.rs` lines 591 to 597

```text
  295 | ) ;
  296 | assert ! ( storage_answers ( & fake ) . is_empty ( ) ) ;
  297 | }
  299 | # [ test ]
  300 | fn host_calls_of_a_creation_that_gives_no_instance_are_dropped ( ) {
  301 | let mut creates =
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 326 to 332, and kopuz-1d8b323 `shared/config/src/store.rs` lines 1217 to 1223

```text
  326 | ) ;
  327 | assert ! ( storage_answers ( & fake ) . is_empty ( ) ) ;
  328 | }
  330 | # [ test ]
  331 | fn an_instance_of_another_interface_is_refused ( ) {
  332 | let fake = Fake
```


### 30 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-1d8b323 `daemon/core/src/auto_sync.rs` lines 257 to 260

```text
    5 | path :: Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync :: mpsc
```


### 30 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and kopuz-1d8b323 `apps/kopuz/link/src/replace.rs` lines 97 to 101

```text
    5 | Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync :: mpsc :: channel
```


### 30 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 13 to 18, and kopuz-1d8b323 `daemon/player/src/engine/mod.rs` lines 20 to 26

```text
   13 | } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time :: Duration ;
   18 | use
```


### 101 normalised tokens: `crates/widevine/src/pssh.rs` lines 33 to 46, and adapted@fce9347 `src_lib.rs` lines 72 to 82

```text
   33 | ) ;
   36 | let size = 32 + data . len ( ) ;
   37 | let mut pssh = Vec :: with_capacity ( size ) ;
   39 | pssh . extend_from_slice ( & ( size as u32 ) . to_be_bytes ( ) ) ;
   40 | pssh . extend_from_slice ( b"pssh" ) ;
   41 | pssh . extend_from_slice ( & [ 0 , 0 , 0 , 0 ] ) ;
   42 | pssh . extend_from_slice ( & SYSTEM_ID ) ;
   43 | pssh . extend_from_slice ( & ( data . len ( ) as u32 ) . to_be_bytes ( ) ) ;
   44 | pssh . extend_from_slice ( & data ) ;
   45 | pssh
   46 | }
```


### 45 normalised tokens: `crates/widevine/src/pssh.rs` lines 4 to 6, and adapted@fce9347 `src_lib.rs` lines 14 to 16

```text
    4 | pub const SYSTEM_ID : [ u8 ; 16 ] = [
    5 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d , 0x21 , 0xed ,
    6 | ] ;
```


### 39 normalised tokens: `crates/widevine/src/find.rs` lines 252 to 256, and adapted@fce9347 `src_lib.rs` lines 130 to 134

```text
  252 | )
  253 | . map ( | entry | entry . path ( ) )
  254 | . filter ( | path | path . is_dir ( ) )
  255 | . collect ( ) ;
  256 | folders . sort_by ( | left , right |
```


### 38 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 49 to 52, and adapted@fce9347 `shim_widevine_shim.cc` lines 278 to 281

```text
   49 | capacity_ ; }
   50 | uint8_t * Data ( ) override { return data_ ; }
   51 | void SetSize ( uint32_t size ) override { size_ = size ; }
   52 | uint32_t Size ( ) const override { return size_ ; }
```


### 35 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 72, and adapted@fce9347 `shim_widevine_shim.cc` lines 279 to 281

```text
   70 | Buffer * DecryptedBuffer ( ) override { return buffer_ ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
```


### 35 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 13 to 16, and adapted@fce9347 `src_native.rs` lines 4 to 7

```text
   13 | std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time ::
```


### 34 normalised tokens: `crates/widevine/src/pssh.rs` lines 59 to 62, and adapted@fce9347 `src_lib.rs` lines 14 to 16

```text
   59 | [
   60 | 0xed , 0xef , 0x8b , 0xa9 , 0x79 , 0xd6 , 0x4a , 0xce , 0xa3 , 0xc8 , 0x27 , 0xdc , 0xd5 , 0x1d ,
   61 | 0x21 , 0xed ,
   62 | ]
```


### 31 normalised tokens: `crates/widevine/shim/cdm_11.cc` lines 70 to 75, and adapted@fce9347 `shim_widevine_shim.cc` lines 61 to 66

```text
   70 | ; }
   71 | void SetTimestamp ( int64_t timestamp ) override { timestamp_ = timestamp ; }
   72 | int64_t Timestamp ( ) const override { return timestamp_ ; }
   74 | private :
   75 | cdm :: Buffer
```


### 31 normalised tokens: `crates/widevine/src/tests/host.rs` lines 4 to 7, and adapted@fce9347 `src_native.rs` lines 5 to 9

```text
    4 | sync :: atomic :: Ordering ;
    5 | use std :: sync :: mpsc :: { Receiver , channel } ;
    6 | use std :: sync :: { Arc , Mutex } ;
    7 | use
```


### 54 normalised tokens: `crates/widevine/src/runtime.rs` lines 198 to 212, and tree `crates/player/src/output.rs` lines 22 to 40

```text
  198 | # [ derive ( Default ) ]
  199 | pub ( crate ) struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
```


### 47 normalised tokens: `crates/widevine/src/host.rs` lines 204 to 214, and tree `crates/player/src/engine.rs` lines 176 to 190

```text
  204 | ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32
```


### 47 normalised tokens: `crates/widevine/src/host.rs` lines 223 to 230, and tree `crates/app-desktop/src/app.rs` lines 958 to 965

```text
  223 | ,
  224 | instance : None ,
  225 | timers : Timers :: default ( ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise :
```


### 47 normalised tokens: `crates/widevine/src/host.rs` lines 224 to 231, and tree `crates/app-desktop/src/app.rs` lines 960 to 967

```text
  224 | ,
  225 | timers : Timers :: default ( ) ,
  226 | sessions : Vec :: new ( ) ,
  227 | waits : Vec :: new ( ) ,
  228 | early_messages : Vec :: new ( ) ,
  229 | later : VecDeque :: new ( ) ,
  230 | last_promise : 0 ,
  231 | subsamples :
```


### 46 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 216, and tree `crates/app-desktop/src/app.rs` lines 276 to 288

```text
  199 | {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
  215 | impl Shared {
  216 | pub
```


### 44 normalised tokens: `crates/widevine/src/host.rs` lines 156 to 160, and tree `crates/app-desktop/src/app.rs` lines 4968 to 4972

```text
  156 | Wait :: Initialise { deadline , .. }
  157 | | Wait :: Open { deadline , .. }
  158 | | Wait :: Licence { deadline , .. }
  159 | | Wait :: Close { deadline , .. }
  160 | | Wait :: AbandonedOpen { deadline , .. }
```


### 44 normalised tokens: `crates/widevine/src/runtime.rs` lines 199 to 213, and tree `crates/opium-core/src/theme.rs` lines 324 to 333

```text
  199 | struct Counters {
  201 | pub turns : AtomicU64 ,
  203 | pub accepted : AtomicU64 ,
  205 | pub answered : AtomicU64 ,
  207 | pub ignored : AtomicU64 ,
  208 | pub timers_set : AtomicU64 ,
  209 | pub timers_fired : AtomicU64 ,
  210 | pub storage_requests : AtomicU64 ,
  212 | pub panicked : AtomicBool ,
  213 | }
```


### 43 normalised tokens: `crates/widevine/src/runtime.rs` lines 25 to 31, and tree `crates/app-desktop/src/ui/menu.rs` lines 402 to 414

```text
   25 | # [ derive ( Debug , Clone , Copy ) ]
   26 | pub ( crate ) struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
```


### 43 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 405 to 416, and tree `crates/opium-core/src/theme.rs` lines 141 to 152

```text
  405 | ,
  406 | create : fake_create ,
  407 | initialize : fake_initialize ,
  408 | create_session : fake_create_session ,
  409 | update_session : fake_update_session ,
  410 | close_session : fake_close_session ,
  411 | timer_expired : fake_timer_expired ,
  412 | decrypt : fake_decrypt ,
  413 | release_buffer : fake_release_buffer ,
  414 | storage_id : fake_storage_id ,
  415 | destroy : fake_destroy ,
  416 | } ;
```


### 38 normalised tokens: `crates/widevine/src/boundary.rs` lines 54 to 61, and tree `crates/cenc/src/table.rs` lines 73 to 80

```text
   54 | # [ derive ( Clone , Copy , Debug , PartialEq , Eq ) ]
   55 | pub ( crate ) struct Subsample {
   56 | pub clear_bytes : u32 ,
   57 | pub cipher_bytes : u32 ,
   58 | }
   61 | # [ repr ( C
```


### 38 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 383 to 388, and tree `crates/desktop/src/backend.rs` lines 1904 to 1909

```text
  383 | Step :: Later { .. }
  384 | | Step :: Mark ( _ )
  385 | | Step :: Status ( _ )
  386 | | Step :: OutputLength ( _ )
  387 | | Step :: Wait ( _ )
  388 | | Step :: Signal
```


### 38 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 11 to 16, and tree `crates/local/src/scan.rs` lines 5 to 9

```text
   11 | sequence ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync :: atomic :: { AtomicU64 , Ordering } ;
   16 | use std :: time ::
```


### 38 normalised tokens: `crates/widevine/src/tests/round_trip.rs` lines 31 to 32, and tree `crates/apple-music/examples/cdm_cost.rs` lines 202 to 203

```text
   31 | . sort_unstable ( ) ;
   32 | let at = | share : f64 | times [ ( ( times . len ( ) - 1 ) as f64 * share ) . round ( ) as usize ] ;
```


### 37 normalised tokens: `crates/widevine/src/boundary.rs` lines 13 to 16, and tree `crates/downloads/src/queue.rs` lines 6 to 10

```text
   13 | use std :: cell :: { Cell , RefCell } ;
   14 | use std :: ffi :: { c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std :: ptr
```


### 36 normalised tokens: `crates/widevine/src/desktop.rs` lines 151 to 161, and tree `crates/opium-core/src/growing.rs` lines 21 to 29

```text
  151 | struct Loaded {
  153 | path : Option < PathBuf > ,
  155 | ready : bool ,
  156 | }
  158 | # [ derive ( Default ) ]
  159 | struct Current {
  160 | runtime : Option < Runtime > ,
  161 | shut_down : bool ,
```


### 36 normalised tokens: `crates/widevine/src/host.rs` lines 202 to 208, and tree `crates/player/src/engine.rs` lines 179 to 192

```text
  202 | ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec
```


### 36 normalised tokens: `crates/widevine/src/host.rs` lines 204 to 209, and tree `crates/app-desktop/src/ui/detail.rs` lines 203 to 209

```text
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec < Wait > ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque <
```


### 36 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 384 to 388, and tree `crates/desktop/src/backend.rs` lines 2571 to 2575

```text
  384 | | Step :: Mark ( _ )
  385 | | Step :: Status ( _ )
  386 | | Step :: OutputLength ( _ )
  387 | | Step :: Wait ( _ )
  388 | | Step :: Signal ( _ ) =>
```


### 36 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 31 to 35, and tree `crates/synced/src/tests.rs` lines 562 to 564

```text
   31 | ,
   32 | std :: process :: id ( )
   33 | ) ) ;
   34 | let _ = std :: fs :: remove_dir_all ( & path ) ;
   35 | std :: fs :: create_dir_all ( & path ) . expect (
```


### 35 normalised tokens: `crates/widevine/src/boundary.rs` lines 210 to 226, and tree `crates/opium-core/src/provider.rs` lines 4 to 8

```text
  210 | set_timer ,
  211 | wall_time ,
  212 | on_initialized ,
  213 | on_resolve_key_status ,
  214 | on_resolve_new_session ,
  215 | on_resolve ,
  216 | on_reject ,
  217 | on_session_message ,
  218 | on_keys_change ,
  219 | on_expiration_change ,
  220 | on_session_closed ,
  221 | request_storage_id ,
  222 | unanswered ,
  223 | } ;
  226 | # [ derive ( Debug , Clone
```


### 35 normalised tokens: `crates/widevine/src/host.rs` lines 200 to 206, and tree `crates/player/src/lib.rs` lines 214 to 223

```text
  200 | struct Host {
  201 | shared : Arc < Shared > ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec
```


### 35 normalised tokens: `crates/widevine/src/host.rs` lines 207 to 219, and tree `crates/app-desktop/src/screenshot.rs` lines 16 to 28

```text
  207 | ,
  208 | early_messages : Vec < EarlyMessage > ,
  209 | later : VecDeque < Later > ,
  210 | last_promise : u32 ,
  212 | subsamples : Vec < Subsample > ,
  214 | unanswered_logged : u32 ,
  215 | }
  217 | impl Host {
  219 | pub
```


### 35 normalised tokens: `crates/widevine/src/tests/cpp_layer.rs` lines 5 to 9, and tree `crates/downloads/src/queue.rs` lines 10 to 13

```text
    5 | use std :: ffi :: { c_char , c_int , c_void } ;
    6 | use std :: sync :: Mutex ;
    8 | use crate :: boundary :: {
    9 | CdmCalls , DecryptInput , DecryptOutput , HOST_CALLS , HostCalls
```


### 34 normalised tokens: `crates/widevine/src/find.rs` lines 68 to 76, and tree `crates/opium-core/src/model.rs` lines 99 to 103

```text
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf ,
   76 | pub platform : Option <
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 25 to 28, and tree `crates/cenc/src/fragment.rs` lines 4 to 8

```text
   25 | :: library :: Library ;
   26 | use crate :: log ;
   27 | use crate :: runtime :: { Reply , Request , SampleRef , SessionKey , Shared } ;
   28 | use crate :: timers :: Timers ;
```


### 34 normalised tokens: `crates/widevine/src/host.rs` lines 288 to 294, and tree `crates/soundcloud/src/lib.rs` lines 184 to 191

```text
  288 | timers . next_due ( ) ) . min ( )
  289 | }
  291 | fn initialising ( & self ) -> bool {
  292 | self . waits
  293 | . iter ( )
  294 | . any ( | wait | matches
```


### 34 normalised tokens: `crates/widevine/src/lib.rs` lines 59 to 67, and tree `crates/connect/src/lib.rs` lines 190 to 198

```text
   59 | PathBuf ,
   61 | pub version : Option < String > ,
   63 | pub set_aside : Option < String > ,
   64 | }
   67 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
```


### 34 normalised tokens: `crates/widevine/src/lib.rs` lines 59 to 68, and tree `crates/local/src/tags.rs` lines 18 to 25

```text
   59 | ,
   61 | pub version : Option < String > ,
   63 | pub set_aside : Option < String > ,
   64 | }
   67 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq ) ]
   68 | pub
```


### 34 normalised tokens: `crates/widevine/src/runtime.rs` lines 10 to 12, and tree `crates/cenc/tests/allocation.rs` lines 5 to 7

```text
   10 | sync :: atomic :: { AtomicBool , AtomicU64 , Ordering } ;
   11 | use std :: sync :: mpsc :: { SyncSender , sync_channel } ;
   12 | use std :: sync :: { Arc , Condvar
```


### 34 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 57 to 62, and tree `crates/apple-music/src/types.rs` lines 95 to 100

```text
   57 | struct DecryptCall {
   58 | pub size : usize ,
   59 | pub scheme : u32 ,
   60 | pub key_id : Vec < u8 > ,
   61 | pub iv : Vec < u8 > ,
   62 | pub subsamples : Vec <
```


### 34 normalised tokens: `crates/widevine/src/tests/host.rs` lines 4 to 7, and tree `crates/browser-signin/src/lib.rs` lines 15 to 18

```text
    4 | sync :: atomic :: Ordering ;
    5 | use std :: sync :: mpsc :: { Receiver , channel } ;
    6 | use std :: sync :: { Arc , Mutex } ;
    7 | use std :: thread
```


### 34 normalised tokens: `crates/widevine/src/tests/sequence.rs` lines 13 to 16, and tree `crates/app-desktop/src/ui/lyrics.rs` lines 9 to 13

```text
   13 | :: { Rng , key , start } ;
   14 | use crate :: boundary :: { MESSAGE_INDIVIDUALIZATION_REQUEST , MESSAGE_LICENCE_REQUEST } ;
   15 | use crate :: runtime :: { Deadlines , Runtime } ;
   16 | use crate ::
```


### 33 normalised tokens: `crates/widevine/src/host.rs` lines 201 to 207, and tree `crates/app-desktop/src/screenshot.rs` lines 15 to 24

```text
  201 | ,
  202 | library : Library ,
  203 | inbox : Box < Inbox > ,
  204 | instance : Option < Instance > ,
  205 | timers : Timers ,
  206 | sessions : Vec < SessionEntry > ,
  207 | waits : Vec
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 9 to 11, and tree `crates/player/src/engine.rs` lines 4 to 6

```text
    9 | use std :: collections :: VecDeque ;
   10 | use std :: sync :: atomic :: { AtomicBool , AtomicU64 , Ordering } ;
   11 | use std :: sync :: mpsc :: { SyncSender , sync_channel
```


### 33 normalised tokens: `crates/widevine/src/runtime.rs` lines 22 to 29, and tree `crates/apple-music/src/radio.rs` lines 27 to 35

```text
   22 | ;
   25 | # [ derive ( Debug , Clone , Copy ) ]
   26 | pub ( crate ) struct Deadlines {
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration
```


### 33 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 23 to 33, and tree `crates/desktop/src/backend.rs` lines 78 to 91

```text
   23 | ,
   24 | } ,
   25 | Initialize {
   26 | distinctive_identifier : bool ,
   27 | persistent_state : bool ,
   28 | hw_secure_codecs : bool ,
   29 | } ,
   30 | CreateSession {
   31 | promise : u32 ,
   32 | session_type : u32 ,
   33 | init_data_type : u32 ,
```


### 33 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 29 to 38, and tree `crates/desktop/src/backend.rs` lines 99 to 110

```text
   29 | ,
   30 | CreateSession {
   31 | promise : u32 ,
   32 | session_type : u32 ,
   33 | init_data_type : u32 ,
   34 | init_data : Vec < u8 > ,
   35 | } ,
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec
```


### 33 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 53 to 60, and tree `crates/app-desktop/src/ui/menu.rs` lines 399 to 410

```text
   53 | }
   56 | # [ derive ( Debug , Clone , PartialEq ) ]
   57 | pub ( crate ) struct DecryptCall {
   58 | pub size : usize ,
   59 | pub scheme : u32 ,
   60 | pub key_id : Vec
```


### 33 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 56 to 60, and tree `crates/apple-music/src/radio.rs` lines 30 to 35

```text
   56 | # [ derive ( Debug , Clone , PartialEq ) ]
   57 | pub ( crate ) struct DecryptCall {
   58 | pub size : usize ,
   59 | pub scheme : u32 ,
   60 | pub key_id : Vec <
```


### 33 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 60 to 64, and tree `crates/app-desktop/src/app.rs` lines 734 to 743

```text
   60 | u8 > ,
   61 | pub iv : Vec < u8 > ,
   62 | pub subsamples : Vec < ( u32 , u32 ) > ,
   63 | pub crypt : u32 ,
   64 | pub skip : u32 ,
```


### 33 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 5 to 8, and tree `crates/app-desktop/src/app.rs` lines 3 to 6

```text
    5 | use std :: path :: Path ;
    6 | use std :: sync :: Arc ;
    7 | use std :: sync :: atomic :: { AtomicUsize , Ordering } ;
    8 | use std :: sync :: mpsc
```


### 32 normalised tokens: `crates/widevine/src/boundary.rs` lines 262 to 271, and tree `crates/connect/src/lib.rs` lines 163 to 172

```text
  262 | ,
  263 | ExpirationChanged {
  264 | session : Vec < u8 > ,
  265 | expiry : f64 ,
  266 | } ,
  267 | Closed {
  268 | session : Vec < u8 > ,
  269 | } ,
  270 | StorageIdRequested {
  271 | version : u32
```


### 32 normalised tokens: `crates/widevine/src/desktop.rs` lines 119 to 124, and tree `crates/cenc/src/error.rs` lines 24 to 29

```text
  119 | }
  120 | }
  122 | impl fmt :: Debug for Session {
  123 | fn fmt ( & self , formatter : & mut fmt :: Formatter < '_ > ) -> fmt :: Result {
  124 | formatter
```


### 32 normalised tokens: `crates/widevine/src/find.rs` lines 66 to 74, and tree `crates/cenc/src/tests/build.rs` lines 217 to 223

```text
   66 | struct Places {
   68 | pub explicit : Option < PathBuf > ,
   70 | pub config : Option < PathBuf > ,
   72 | pub home : Option < PathBuf > ,
   74 | pub chrome_install : PathBuf ,
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 7 to 14, and tree `crates/player/src/lib.rs` lines 9 to 16

```text
    7 | mod find ;
    8 | mod host ;
    9 | mod loader ;
   10 | mod round_trip ;
   11 | mod sequence ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc
```


### 32 normalised tokens: `crates/widevine/src/tests/mod.rs` lines 9 to 15, and tree `crates/synced/src/lib.rs` lines 13 to 19

```text
    9 | mod loader ;
   10 | mod round_trip ;
   11 | mod sequence ;
   13 | use std :: path :: { Path , PathBuf } ;
   14 | use std :: sync :: Arc ;
   15 | use std :: sync ::
```


### 31 normalised tokens: `crates/widevine/src/lib.rs` lines 75 to 86, and tree `crates/cenc/src/movie.rs` lines 75 to 86

```text
   75 | ,
   77 | pub scheme : Scheme ,
   78 | }
   81 | # [ derive ( Debug , Clone , Copy , PartialEq , Eq , Hash ) ]
   82 | pub enum Scheme {
   84 | Cenc ,
   86 | Cbcs
```


### 31 normalised tokens: `crates/widevine/src/runtime.rs` lines 293 to 296, and tree `crates/opium-core/src/growing.rs` lines 34 to 37

```text
  293 | {
  294 | let shared = Arc :: new ( Shared {
  295 | queue : Mutex :: new ( Queue :: default ( ) ) ,
  296 | wake : Condvar :: new ( ) ,
```


### 31 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 12 to 15, and tree `crates/app-desktop/src/app.rs` lines 13 to 16

```text
   12 | Condvar , Mutex , MutexGuard , PoisonError } ;
   14 | use crate :: boundary :: { CdmCalls , DecryptInput , DecryptOutput , HostCalls , KeyInfo } ;
   15 | use crate :: library ::
```


### 30 normalised tokens: `crates/widevine/src/boundary.rs` lines 14 to 17, and tree `crates/desktop/src/backend/downloading.rs` lines 11 to 14

```text
   14 | c_char , c_void } ;
   15 | use std :: panic :: { AssertUnwindSafe , catch_unwind } ;
   16 | use std :: ptr ;
   17 | use std :: time :: { SystemTime , UNIX_EPOCH
```


### 30 normalised tokens: `crates/widevine/src/desktop.rs` lines 4 to 8, and tree `crates/soundcloud/src/stream.rs` lines 13 to 16

```text
    4 | use std :: fmt ;
    5 | use std :: path :: { Path , PathBuf } ;
    6 | use std :: sync :: { Mutex , MutexGuard , PoisonError } ;
    8 | use
```


### 30 normalised tokens: `crates/widevine/src/desktop.rs` lines 6 to 10, and tree `crates/app-desktop/src/ui/dock.rs` lines 11 to 15

```text
    6 | MutexGuard , PoisonError } ;
    8 | use crate :: library :: { self , Library } ;
    9 | use crate :: runtime :: { Deadlines , Runtime , SessionKey } ;
   10 | use
```


### 30 normalised tokens: `crates/widevine/src/runtime.rs` lines 27 to 36, and tree `crates/app-desktop/src/app.rs` lines 280 to 288

```text
   27 | pub initialise : Duration ,
   28 | pub session_request : Duration ,
   29 | pub licence : Duration ,
   30 | pub close : Duration ,
   31 | pub shutdown : Duration ,
   32 | }
   34 | impl Deadlines {
   36 | pub
```


### 30 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 10 to 12, and tree `crates/browser-signin/src/lib.rs` lines 20 to 22

```text
   10 | sync :: atomic :: { AtomicBool , AtomicI64 , Ordering } ;
   11 | use std :: sync :: mpsc :: Sender ;
   12 | use std :: sync :: { Arc , Condvar
```


### 30 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 32 to 38, and tree `crates/desktop/src/backend.rs` lines 109 to 116

```text
   32 | session_type : u32 ,
   33 | init_data_type : u32 ,
   34 | init_data : Vec < u8 > ,
   35 | } ,
   36 | UpdateSession {
   37 | promise : u32 ,
   38 | session : Vec < u8 > ,
```


### 30 normalised tokens: `crates/widevine/src/tests/fake.rs` lines 120 to 123, and tree `crates/cenc/src/claims.rs` lines 15 to 19

```text
  120 | # [ derive ( Debug , Default ) ]
  121 | pub ( crate ) struct Gate {
  122 | state : Mutex < ( bool , bool ) > ,
  123 | changed : Condvar
```


### 30 normalised tokens: `crates/widevine/src/tests/find.rs` lines 9 to 12, and tree `crates/cenc/src/fragment.rs` lines 4 to 8

```text
    9 | ;
   10 | use crate :: Candidate ;
   11 | use crate :: find :: { Places , candidates_in , choose , compare_versions , version_parts } ;
   12 | use crate :: manifest :: manifest_version ;
```


### 30 normalised tokens: `crates/widevine/src/tests/host.rs` lines 5 to 8, and tree `crates/desktop/src/backend/downloading.rs` lines 11 to 14

```text
    5 | Receiver , channel } ;
    6 | use std :: sync :: { Arc , Mutex } ;
    7 | use std :: thread ;
    8 | use std :: time :: { Duration , Instant
```


### 30 normalised tokens: `crates/widevine/src/tests/loader.rs` lines 276 to 278, and tree `crates/cenc/src/tests/other.rs` lines 131 to 133

```text
  276 | send_and_sync :: < crate :: Cdm > ( ) ;
  277 | send_and_sync :: < crate :: Session > ( ) ;
  278 | send_and_sync :: < crate :: Error > ( ) ;
```


## Names, strings and comments of the sources found in the new code


### kopuz-a58c385 widevine_shim.cc: `widevine_shim.cc`

- Compound identifiers of 6 characters or more: 98; found in the new code: 65.
  `CloseSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `ContentDecryptionModule_11` (cdm_11.cc), `CreateFileIO` (cdm_11.cc), `CreateSessionAndGenerateRequest` (cdm_11.cc), `DecryptedBlock` (cdm_11.cc), `DecryptedBuffer` (cdm_11.cc), `EnableOutputProtection` (cdm_11.cc), `EncryptionScheme` (cdm_11.cc), `FileIO` (cdm_11.cc), `FileIOClient` (cdm_11.cc), `GetCurrentWallTime` (cdm_11.cc), `GetHost` (cdm_11.cc), `Host_11` (cdm_11.cc), `InitDataType` (cdm_11.cc), `InputBuffer_2` (cdm_11.cc), `KeyInformation` (cdm_11.cc), `KeyStatus` (cdm_11.cc, host.rs), `MessageType` (cdm_11.cc), `MetricName` (cdm_11.cc), `OnDeferredInitializationDone` (cdm_11.cc), `OnExpirationChange` (cdm_11.cc), `OnInitialized` (cdm_11.cc), `OnRejectPromise` (cdm_11.cc), `OnResolveKeyStatusPromise` (cdm_11.cc), `OnResolveNewSessionPromise` (cdm_11.cc), `OnResolvePromise` (cdm_11.cc), `OnSessionClosed` (cdm_11.cc), `OnSessionKeysChange` (cdm_11.cc), `OnSessionMessage` (cdm_11.cc), `QueryOutputProtectionStatus` (cdm_11.cc), `RTLD_NOW` (library.rs), `ReportMetrics` (cdm_11.cc), `RequestStorageId` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `SendPlatformChallenge` (cdm_11.cc), `SessionType` (cdm_11.cc), `SetDecryptedBuffer` (cdm_11.cc), `SetSize` (cdm_11.cc), `SetTimer` (cdm_11.cc), `SetTimestamp` (cdm_11.cc), `StreamType` (cdm_11.cc), `SubsampleEntry` (cdm_11.cc), `TimerExpired` (cdm_11.cc), `UpdateSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `cipher_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `clear_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `delay_ms` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, host.rs, sequence.rs, timers.rs), `encryption_scheme` (cdm_11.cc), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `int64_t` (boundary.h, cdm_11.cc), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `kVersion` (cdm_11.cc), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `keys_changed` (host.rs), `message_type` (boundary.h, boundary.rs, cdm_11.cc), `num_subsamples` (cdm_11.cc), `push_back` (host.rs, runtime.rs), `reinterpret_cast` (cdm_11.cc), `session_closed` (host.rs), `session_id` (boundary.h, boundary.rs, cdm_11.cc), `static_cast` (cdm_11.cc), `uint32_t` (boundary.h, cdm_11.cc), `uint64_t` (cdm_11.cc), `uint8_t` (boundary.h, cdm_11.cc)
- String literals of three words or more: 0; found: 0.
- Comment phrases of six words: 159; found: 0.

### kopuz-a58c385 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 73; found in the new code: 27.
  `MutexGuard` (desktop.rs, fake.rs, runtime.rs), `_key_id` (android.rs), `as_ptr` (cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs, runtime.rs), `as_ref` (find.rs, find.rs, host.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `c_char` (boundary.rs, cpp_layer.rs, fake.rs, library.rs), `close_session` (boundary.h, boundary.rs, fake.rs, host.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `extend_from_slice` (pssh.rs), `from_raw_parts` (boundary.rs, cpp_layer.rs, fake.rs, runtime.rs), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `into_inner` (desktop.rs, fake.rs, host.rs, runtime.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `is_null` (boundary.rs, cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `map_err` (library.rs, runtime.rs), `null_mut` (cpp_layer.rs, fake.rs, host.rs, host.rs, round_trip.rs), `ok_or_else` (library.rs), `target_os` (error.rs, lib.rs), `to_be_bytes` (pssh.rs), `to_str` (library.rs), `to_string` (host.rs, host.rs, library.rs, runtime.rs), `to_vec` (boundary.rs, cpp_layer.rs, fake.rs, find.rs, host.rs, round_trip.rs, runtime.rs, sequence.rs), `unwrap_or_else` (desktop.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, runtime.rs), `with_capacity` (pssh.rs, round_trip.rs)
- String literals of three words or more: 39; found: 0.
- Comment phrases of six words: 545; found: 1.
  - "the cdm has no device certificate"

### kopuz-a58c385 discover.rs: `discover.rs`

- Compound identifiers of 6 characters or more: 32; found in the new code: 18.
  `PathBuf` (android.rs, desktop.rs, error.rs, find.rs, find.rs, lib.rs, loader.rs, mod.rs, probe.rs), `as_os_str` (find.rs, library.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `create_dir_all` (find.rs, mod.rs), `file_name` (find.rs), `filter_map` (find.rs, host.rs), `is_ascii_digit` (find.rs), `is_dir` (find.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `is_file` (find.rs), `or_else` (find.rs, probe.rs, runtime.rs), `read_dir` (find.rs), `remove_dir_all` (mod.rs), `target_os` (error.rs, lib.rs), `temp_dir` (mod.rs), `to_str` (library.rs), `unwrap_or_default` (find.rs, host.rs), `var_os` (find.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 131; found: 0.

### kopuz-a58c385 fetch.rs: `fetch.rs`

- Compound identifiers of 6 characters or more: 105; found in the new code: 30.
  `AtomicBool` (fake.rs, runtime.rs), `PartialEq` (boundary.rs, cpp_layer.rs, error.rs, fake.rs, host.rs, lib.rs, timers.rs), `PathBuf` (android.rs, desktop.rs, error.rs, find.rs, find.rs, lib.rs, loader.rs, mod.rs, probe.rs), `and_then` (find.rs), `as_deref` (build.rs, find.rs, find.rs, host.rs, probe.rs), `as_ref` (find.rs, find.rs, host.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `checked_add` (timers.rs), `create_dir_all` (find.rs, mod.rs), `extend_from_slice` (pssh.rs), `file_name` (find.rs), `find_map` (host.rs), `from_utf8_lossy` (boundary.rs, manifest.rs), `into_owned` (boundary.rs, find.rs, library.rs), `is_err` (runtime.rs), `is_file` (find.rs), `is_none` (find.rs, host.rs, loader.rs, runtime.rs), `is_some` (find.rs, host.rs, runtime.rs), `map_err` (library.rs, runtime.rs), `ok_or_else` (library.rs), `or_else` (find.rs, probe.rs, runtime.rs), `read_dir` (find.rs), `remove_dir_all` (mod.rs), `rsplit_once` (find.rs), `starts_with` (find.rs), `target_os` (error.rs, lib.rs), `to_string` (host.rs, host.rs, library.rs, runtime.rs), `to_string_lossy` (find.rs, find.rs, library.rs), `to_vec` (boundary.rs, cpp_layer.rs, fake.rs, find.rs, host.rs, round_trip.rs, runtime.rs, sequence.rs), `unwrap_or_default` (find.rs, host.rs)
- String literals of three words or more: 62; found: 0.
- Comment phrases of six words: 524; found: 0.

### kopuz-a58c385 mediadrm.rs: `mediadrm.rs`

- Compound identifiers of 6 characters or more: 24; found in the new code: 7.
  `PartialEq` (boundary.rs, cpp_layer.rs, error.rs, fake.rs, host.rs, lib.rs, timers.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `clear_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `copy_from_slice` (round_trip.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `subsample_count` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs, runtime.rs), `to_be_bytes` (pssh.rs)
- String literals of three words or more: 1; found: 0.
- Comment phrases of six words: 152; found: 0.

### kopuz-1d8b323 widevine_shim.cc: `widevine_shim.cc`

- Compound identifiers of 6 characters or more: 98; found in the new code: 65.
  `CloseSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `ContentDecryptionModule_11` (cdm_11.cc), `CreateFileIO` (cdm_11.cc), `CreateSessionAndGenerateRequest` (cdm_11.cc), `DecryptedBlock` (cdm_11.cc), `DecryptedBuffer` (cdm_11.cc), `EnableOutputProtection` (cdm_11.cc), `EncryptionScheme` (cdm_11.cc), `FileIO` (cdm_11.cc), `FileIOClient` (cdm_11.cc), `GetCurrentWallTime` (cdm_11.cc), `GetHost` (cdm_11.cc), `Host_11` (cdm_11.cc), `InitDataType` (cdm_11.cc), `InputBuffer_2` (cdm_11.cc), `KeyInformation` (cdm_11.cc), `KeyStatus` (cdm_11.cc, host.rs), `MessageType` (cdm_11.cc), `MetricName` (cdm_11.cc), `OnDeferredInitializationDone` (cdm_11.cc), `OnExpirationChange` (cdm_11.cc), `OnInitialized` (cdm_11.cc), `OnRejectPromise` (cdm_11.cc), `OnResolveKeyStatusPromise` (cdm_11.cc), `OnResolveNewSessionPromise` (cdm_11.cc), `OnResolvePromise` (cdm_11.cc), `OnSessionClosed` (cdm_11.cc), `OnSessionKeysChange` (cdm_11.cc), `OnSessionMessage` (cdm_11.cc), `QueryOutputProtectionStatus` (cdm_11.cc), `RTLD_NOW` (library.rs), `ReportMetrics` (cdm_11.cc), `RequestStorageId` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `SendPlatformChallenge` (cdm_11.cc), `SessionType` (cdm_11.cc), `SetDecryptedBuffer` (cdm_11.cc), `SetSize` (cdm_11.cc), `SetTimer` (cdm_11.cc), `SetTimestamp` (cdm_11.cc), `StreamType` (cdm_11.cc), `SubsampleEntry` (cdm_11.cc), `TimerExpired` (cdm_11.cc), `UpdateSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `cipher_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `clear_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `delay_ms` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, host.rs, sequence.rs, timers.rs), `encryption_scheme` (cdm_11.cc), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `int64_t` (boundary.h, cdm_11.cc), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `kVersion` (cdm_11.cc), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `keys_changed` (host.rs), `message_type` (boundary.h, boundary.rs, cdm_11.cc), `num_subsamples` (cdm_11.cc), `push_back` (host.rs, runtime.rs), `reinterpret_cast` (cdm_11.cc), `session_closed` (host.rs), `session_id` (boundary.h, boundary.rs, cdm_11.cc), `static_cast` (cdm_11.cc), `uint32_t` (boundary.h, cdm_11.cc), `uint64_t` (cdm_11.cc), `uint8_t` (boundary.h, cdm_11.cc)
- String literals of three words or more: 0; found: 0.
- Comment phrases of six words: 159; found: 0.

### kopuz-1d8b323 mod.rs: `mod.rs`

- Compound identifiers of 6 characters or more: 73; found in the new code: 27.
  `MutexGuard` (desktop.rs, fake.rs, runtime.rs), `_key_id` (android.rs), `as_ptr` (cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs, runtime.rs), `as_ref` (find.rs, find.rs, host.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `c_char` (boundary.rs, cpp_layer.rs, fake.rs, library.rs), `close_session` (boundary.h, boundary.rs, fake.rs, host.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `extend_from_slice` (pssh.rs), `from_raw_parts` (boundary.rs, cpp_layer.rs, fake.rs, runtime.rs), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `into_inner` (desktop.rs, fake.rs, host.rs, runtime.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `is_null` (boundary.rs, cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `map_err` (library.rs, runtime.rs), `null_mut` (cpp_layer.rs, fake.rs, host.rs, host.rs, round_trip.rs), `ok_or_else` (library.rs), `target_os` (error.rs, lib.rs), `to_be_bytes` (pssh.rs), `to_str` (library.rs), `to_string` (host.rs, host.rs, library.rs, runtime.rs), `to_vec` (boundary.rs, cpp_layer.rs, fake.rs, find.rs, host.rs, round_trip.rs, runtime.rs, sequence.rs), `unwrap_or_else` (desktop.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, runtime.rs), `with_capacity` (pssh.rs, round_trip.rs)
- String literals of three words or more: 39; found: 0.
- Comment phrases of six words: 545; found: 1.
  - "the cdm has no device certificate"

### kopuz-1d8b323 discover.rs: `discover.rs`

- Compound identifiers of 6 characters or more: 32; found in the new code: 18.
  `PathBuf` (android.rs, desktop.rs, error.rs, find.rs, find.rs, lib.rs, loader.rs, mod.rs, probe.rs), `as_os_str` (find.rs, library.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `create_dir_all` (find.rs, mod.rs), `file_name` (find.rs), `filter_map` (find.rs, host.rs), `is_ascii_digit` (find.rs), `is_dir` (find.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `is_file` (find.rs), `or_else` (find.rs, probe.rs, runtime.rs), `read_dir` (find.rs), `remove_dir_all` (mod.rs), `target_os` (error.rs, lib.rs), `temp_dir` (mod.rs), `to_str` (library.rs), `unwrap_or_default` (find.rs, host.rs), `var_os` (find.rs)
- String literals of three words or more: 5; found: 0.
- Comment phrases of six words: 131; found: 0.

### kopuz-1d8b323 fetch.rs: `fetch.rs`

- Compound identifiers of 6 characters or more: 105; found in the new code: 30.
  `AtomicBool` (fake.rs, runtime.rs), `PartialEq` (boundary.rs, cpp_layer.rs, error.rs, fake.rs, host.rs, lib.rs, timers.rs), `PathBuf` (android.rs, desktop.rs, error.rs, find.rs, find.rs, lib.rs, loader.rs, mod.rs, probe.rs), `and_then` (find.rs), `as_deref` (build.rs, find.rs, find.rs, host.rs, probe.rs), `as_ref` (find.rs, find.rs, host.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `checked_add` (timers.rs), `create_dir_all` (find.rs, mod.rs), `extend_from_slice` (pssh.rs), `file_name` (find.rs), `find_map` (host.rs), `from_utf8_lossy` (boundary.rs, manifest.rs), `into_owned` (boundary.rs, find.rs, library.rs), `is_err` (runtime.rs), `is_file` (find.rs), `is_none` (find.rs, host.rs, loader.rs, runtime.rs), `is_some` (find.rs, host.rs, runtime.rs), `map_err` (library.rs, runtime.rs), `ok_or_else` (library.rs), `or_else` (find.rs, probe.rs, runtime.rs), `read_dir` (find.rs), `remove_dir_all` (mod.rs), `rsplit_once` (find.rs), `starts_with` (find.rs), `target_os` (error.rs, lib.rs), `to_string` (host.rs, host.rs, library.rs, runtime.rs), `to_string_lossy` (find.rs, find.rs, library.rs), `to_vec` (boundary.rs, cpp_layer.rs, fake.rs, find.rs, host.rs, round_trip.rs, runtime.rs, sequence.rs), `unwrap_or_default` (find.rs, host.rs)
- String literals of three words or more: 62; found: 0.
- Comment phrases of six words: 524; found: 0.

### kopuz-1d8b323 mediadrm.rs: `mediadrm.rs`

- Compound identifiers of 6 characters or more: 24; found in the new code: 7.
  `PartialEq` (boundary.rs, cpp_layer.rs, error.rs, fake.rs, host.rs, lib.rs, timers.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `clear_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `copy_from_slice` (round_trip.rs), `is_empty` (boundary.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, library.rs, loader.rs, runtime.rs, sequence.rs), `subsample_count` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs, runtime.rs), `to_be_bytes` (pssh.rs)
- String literals of three words or more: 1; found: 0.
- Comment phrases of six words: 152; found: 0.

### fce9347 build.rs: `build.rs`

- Compound identifiers of 6 characters or more: 3; found in the new code: 2.
  `as_deref` (build.rs, find.rs, find.rs, host.rs, probe.rs), `opt_level` (build.rs)
- String literals of three words or more: 0; found: 0.
- Comment phrases of six words: 16; found: 0.

### fce9347 examples_probe.rs: `examples_probe.rs`

- Compound identifiers of 6 characters or more: 2; found in the new code: 0.
- String literals of three words or more: 2; found: 0.
- Comment phrases of six words: 10; found: 1.
  - "cargo run -p widevine --example probe"

### fce9347 shim_widevine_shim.cc: `shim_widevine_shim.cc`

- Compound identifiers of 6 characters or more: 103; found in the new code: 67.
  `CloseSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `ContentDecryptionModule_11` (cdm_11.cc), `CreateFileIO` (cdm_11.cc), `CreateSessionAndGenerateRequest` (cdm_11.cc), `DecryptedBlock` (cdm_11.cc), `DecryptedBuffer` (cdm_11.cc), `EnableOutputProtection` (cdm_11.cc), `EncryptionScheme` (cdm_11.cc), `FileIO` (cdm_11.cc), `FileIOClient` (cdm_11.cc), `GetCurrentWallTime` (cdm_11.cc), `GetHost` (cdm_11.cc), `Host_11` (cdm_11.cc), `InitDataType` (cdm_11.cc), `InputBuffer_2` (cdm_11.cc), `KeyInformation` (cdm_11.cc), `KeyStatus` (cdm_11.cc, host.rs), `MessageType` (cdm_11.cc), `MetricName` (cdm_11.cc), `OnDeferredInitializationDone` (cdm_11.cc), `OnExpirationChange` (cdm_11.cc), `OnInitialized` (cdm_11.cc), `OnRejectPromise` (cdm_11.cc), `OnResolveKeyStatusPromise` (cdm_11.cc), `OnResolveNewSessionPromise` (cdm_11.cc), `OnResolvePromise` (cdm_11.cc), `OnSessionClosed` (cdm_11.cc), `OnSessionKeysChange` (cdm_11.cc), `OnSessionMessage` (cdm_11.cc), `QueryOutputProtectionStatus` (cdm_11.cc), `RTLD_NOW` (library.rs), `ReportMetrics` (cdm_11.cc), `RequestStorageId` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `SendPlatformChallenge` (cdm_11.cc), `SessionType` (cdm_11.cc), `SetDecryptedBuffer` (cdm_11.cc), `SetSize` (cdm_11.cc), `SetTimer` (cdm_11.cc), `SetTimestamp` (cdm_11.cc), `StreamType` (cdm_11.cc), `SubsampleEntry` (cdm_11.cc), `TimerExpired` (cdm_11.cc), `UpdateSession` (cdm_11.cc, fake.rs, host.rs, sequence.rs), `cipher_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `clear_bytes` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `crypt_byte_block` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `delay_ms` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, host.rs, sequence.rs, timers.rs), `encryption_scheme` (cdm_11.cc), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `int64_t` (boundary.h, cdm_11.cc), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `kVersion` (cdm_11.cc), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `keys_changed` (host.rs), `message_type` (boundary.h, boundary.rs, cdm_11.cc), `num_subsamples` (cdm_11.cc), `push_back` (host.rs, runtime.rs), `reinterpret_cast` (cdm_11.cc), `session_closed` (host.rs), `session_id` (boundary.h, boundary.rs, cdm_11.cc), `skip_byte_block` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `static_cast` (cdm_11.cc), `uint32_t` (boundary.h, cdm_11.cc), `uint64_t` (cdm_11.cc), `uint8_t` (boundary.h, cdm_11.cc)
- String literals of three words or more: 0; found: 0.
- Comment phrases of six words: 178; found: 0.

### fce9347 src_lib.rs: `src_lib.rs`

- Compound identifiers of 6 characters or more: 32; found in the new code: 20.
  `NotACdm` (error.rs, library.rs, loader.rs), `PathBuf` (android.rs, desktop.rs, error.rs, find.rs, find.rs, lib.rs, loader.rs, mod.rs, probe.rs), `SYSTEM_ID` (lib.rs, pssh.rs), `assert_eq` (cpp_layer.rs, find.rs, host.rs, library.rs, loader.rs, manifest.rs, pssh.rs, round_trip.rs, sequence.rs, timers.rs), `create_dir_all` (find.rs, mod.rs), `extend_from_slice` (pssh.rs), `into_iter` (find.rs, find.rs, host.rs, host.rs, sequence.rs), `is_dir` (find.rs), `is_file` (find.rs), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `read_dir` (find.rs), `remove_dir_all` (mod.rs), `sort_by` (find.rs), `target_os` (error.rs, lib.rs), `temp_dir` (mod.rs), `to_be_bytes` (pssh.rs), `to_owned` (fake.rs, find.rs, host.rs, host.rs, library.rs, runtime.rs, sequence.rs), `unwrap_or_else` (desktop.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, runtime.rs), `var_os` (find.rs), `with_capacity` (pssh.rs, round_trip.rs)
- String literals of three words or more: 9; found: 0.
- Comment phrases of six words: 79; found: 0.

### fce9347 src_native.rs: `src_native.rs`

- Compound identifiers of 6 characters or more: 54; found in the new code: 22.
  `AtomicUsize` (loader.rs), `MutexGuard` (desktop.rs, fake.rs, runtime.rs), `NotACdm` (error.rs, library.rs, loader.rs), `as_os_str` (find.rs, library.rs), `as_ptr` (cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs, runtime.rs), `c_char` (boundary.rs, cpp_layer.rs, fake.rs, library.rs), `data_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `fetch_add` (fake.rs, host.rs, loader.rs, mod.rs, runtime.rs), `fetch_sub` (fake.rs), `from_millis` (host.rs, mod.rs, sequence.rs, timers.rs), `from_raw_parts` (boundary.rs, cpp_layer.rs, fake.rs, runtime.rs), `init_data` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, probe.rs, runtime.rs), `into_inner` (desktop.rs, fake.rs, host.rs, runtime.rs), `is_null` (boundary.rs, cpp_layer.rs, fake.rs, host.rs, library.rs, round_trip.rs), `iv_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `key_id` (boundary.h, boundary.rs, cdm_11.cc, desktop.rs, fake.rs, host.rs, host.rs, lib.rs, pssh.rs, round_trip.rs, runtime.rs, sequence.rs), `key_id_size` (boundary.h, boundary.rs, cdm_11.cc, fake.rs, host.rs, round_trip.rs), `map_err` (library.rs, runtime.rs), `null_mut` (cpp_layer.rs, fake.rs, host.rs, host.rs, round_trip.rs), `to_owned` (fake.rs, find.rs, host.rs, host.rs, library.rs, runtime.rs, sequence.rs), `to_vec` (boundary.rs, cpp_layer.rs, fake.rs, find.rs, host.rs, round_trip.rs, runtime.rs, sequence.rs), `unwrap_or_else` (desktop.rs, fake.rs, find.rs, find.rs, host.rs, host.rs, runtime.rs)
- String literals of three words or more: 1; found: 0.
- Comment phrases of six words: 158; found: 0.

### fce9347 src_stub.rs: `src_stub.rs`

- Compound identifiers of 6 characters or more: 5; found in the new code: 2.
  `_key_id` (android.rs), `to_owned` (fake.rs, find.rs, host.rs, host.rs, library.rs, runtime.rs, sequence.rs)
- String literals of three words or more: 0; found: 0.
- Comment phrases of six words: 17; found: 0.
