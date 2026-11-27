# AGENTS.md — VIctory fork, contratto operativo dell'agente

Questo file è il contratto di ogni sessione (umana o agente) su questo repository. Si legge per intero
prima di agire. Lo stato vivo è in `STATE.md` (fuori da git); questo file cambia solo quando cambia il metodo.

## 1. Obiettivo (unico, misurabile)

Il titolo Xbox Series su PC **come retail, senza compromessi**:
- **G1 Draw corrette:** ogni passata disegnata come su console. Niente nero in ombra, niente sovraesposizione,
  niente geometria mancante, niente artefatti ai bordi dei triangoli, HUD e testi completi.
- **G2 Shader perfetti:** ogni shader tradotto, validato, eseguito con lo stesso risultato della console.
- **G3 30 fps** stabili nel mondo (33 ms per frame, p95 ≤ 40 ms) su RTX 4080 16 GB e 7800X3D.
- **G4 Esperienza completa:** dal boot alla storia senza crash; input, audio e salvataggi funzionanti.

Ogni lavoro deve spostare uno di G1-G4 **in una misura** (immagine o numero). Se non sposta niente, non si fa.

**Condizioni fisse della North Star (decisione dell'Host, non negoziabili):** Series X, uscita 4K, nessun
abbassamento di risoluzione interna (mai Full HD/1080p), mai `--console series-s`. Una run fuori da queste
condizioni non si lancia nemmeno come sonda.

**Il fine prevale sul metodo.** Se un passo di questo loop allontana da G1-G4 o rallenta il percorso più corto
verso di essi, il passo è sbagliato: si dice all'Host e si corregge questo file. Mai scegliere il passo piccolo e
sicuro quando il passo grosso è quello che serve.

## 2. Il loop (ogni ciclo, nell'ordine; nessuna fase si salta)

0. **BASE PER MISURA.** La base è il ramo che, misurato qui nelle condizioni fisse (§1, §3), è più avanti su G1-G4.
   Un fork che si dichiara più avanti si compila puro (fuori dal repo, `D:\Temp\<fork>`), gira col **suo** set e
   poi col **nostro**, affiancato al nostro main: se è avanti, diventa la base e se ne fondono **tutti** i commit
   (merge, non pezzi; un commit che rompe si corregge in avanti). Misura del 2026-11-27 (STATE nodo 232): Cool Fox
   col suo set si blocca prima del mondo, col nostro fa 14,9 fps contro 18,4 e perde i capelli: **base = il nostro
   main**. Dai donatori si prende il pezzo che sposta una misura **qui**; i pezzi provati su AMD si portano e si
   rendono corretti su NVIDIA (wave32, 4080) prima di giudicarli.
1. **ALIGN: SOSPESO** (Host, 2026-11-27). Upstream 5dbf5ce22 ha sostituito il suo albero con trustno1 VIctory-NVIDIA,
   misurato qui peggio (Lucia senza testa). origin/main non si fonde più in automatico: è un donatore come gli
   altri (§2 passo 4), il nostro main va avanti da solo. Niente cron di `victory-align.sh`.
2. **SCAN.** Esegui `bash /d/Tools/victory-forkscan.sh`: stampa i commit nuovi di ogni fork della famiglia
   (§6) dall'ultima scansione. Cron ogni ora (`41 * * * *`). Serve a: (a) accorgersi se un fork dichiara di essere
   più avanti della base (allora si misura, passo 0); (b) trovare il pezzo che attacca il **muro misurato** di oggi
   (STATE.md, PROSSIMO) o un difetto **visibile da noi**. In coda entra solo ciò che passa (a) o (b), con evidenza.
3. **PICK.** Priorità: crash/blocco > muro di G3 misurato > G1 visibile nella nostra vista > resto. Un pezzo che non
   tocca il muro di oggi (es. lane CPU quando il muro è la scheda) non si porta per G3.
4. **PORT / FIX.** Un pezzo per commit, con `git cherry-pick -x <sha>` (oppure riscrittura manuale). Crediti nel
   messaggio: `(port of <Fork> <sha>, GPL-3.0)`. Mai mescolare due pezzi in un commit.
5. **BUILD + TEST + PRECOMPILE.** Build diag (§4). `cargo test` sul crate toccato. Se non compila, il pezzo non
   entra. Poi **sempre** `warm.ps1 -Set "<set della run>"` (shader + pipeline, §2b E1): senza, run.ps1 rifiuta.
6. **RUN.** Una run standard (§3), una variabile alla volta rispetto al riferimento (o un pacchetto di leve
   piccole, §2b E5). Dopo un cambio di traduzione la prima run è `-WarmUp`, la misura è la seconda.
7. **JUDGE.** Si decide sulle immagini affiancate, agli stessi secondi, e sui numeri di `fps.py`, a parità di
   stato della libreria PSO: solo run con `VERDICT: MEASUREMENT valid` (made=0), come il riferimento.
   Migliora → resta. Uguale ma corretto → resta. Peggiora o dubbio → si cerca la causa; `git revert` solo con la
   causa annotata.
8. **COMMIT + STATE.** Commit locale. STATE.md: FATTO (un nodo), coda aggiornata, PROSSIMO (uno solo).
   Ai checkpoint, push del ramo di integrazione sul fork nostro (`hk`), mai su origin.

**Determinismo.** Stesso input, stessa vista, stessi secondi, stesso intervallo di cattura. Non si dichiara
niente (mondo, fps, fix) senza uno screenshot e un log **di quella run**. Le ipotesi si scrivono con la loro
evidenza (codice, disassembly, log, misura del donatore). "Proviamo" non è un metodo: prima la causa, poi il fix.

## 2b. Guardie anti-errore (obbligatorie; ognuna nasce da un errore vero, data e run fra parentesi)

Un errore non si scusa: si rende impossibile. Dove si può la guardia è **meccanica** (lo script rifiuta); dove
non si può è una domanda a cui si risponde per iscritto **prima** dell'azione. Una risposta che manca = non si agisce.

- **E1 CAMBIO DI TRADUZIONE = PRECOMPILA TUTTO** (avmscc1, 2026-11-27: bump a v8 e run subito, 0,2 fps per 140 s,
  presentata come verifica). Cambio di `TRANSLATOR_VERSION`, del codice del traduttore, di uno switch nella chiave
  di traduzione (`shader_cache.rs` `build()`: fra gli altri `VICTORY_EARLY_DEPTH`, `VICTORY_WAVE_PAIRS_ONLY`,
  `VICTORY_THREAD_TICKET`, `VICTORY_SHADER_OPT`), o build nuova: `warm.ps1 -Set "<set>"` (= `victory shaders` +
  `victory warm --used`, cache del driver, poi timbro exe+set), poi **una** run `-WarmUp` (riempie la libreria
  `host-pipelines-<chiave>.bin`), poi la misura. **Meccanico:** run.ps1 rifiuta senza timbro per quell'exe e quel
  set, e rifiuta un exe più vecchio dei sorgenti.
- **E2 IL VERDETTO È DELLO SCRIPT** (avmscc2: 317 pipeline costruite nella run, giudicata contro avmv7b che ne aveva
  0). Un numero si riferisce all'Host solo con `VERDICT: MEASUREMENT valid` stampato da run.ps1 (somma dei `made=`
  di "host pipelines all made" = 0). Altrimenti la run si ripete, non si commenta.
- **E3 DEFAULT DEI TOOL LETTI NEL SORGENTE** (replay `--retranslate`: host-wave di default 64 = RDNA, 127 shader
  rifiutati, primo A/B nullo). Prima di usare un'opzione per una misura si legge il suo default. Trappole note:
  replay `--host-wave 32` sempre; replay e `victory` con cwd = checkout (victory.toml); exe dal repo non scrive fuori.
- **E4 ARCHIVIO PRIMA DELL'ESPERIMENTO** (WAVE_PAIRS 1f1fbd86 e `--one-submit` rimisurati da capo). Prima di ogni
  esperimento: `grep -n <hash|switch|idea> STATE.md STATE-archive.md`; se c'è, si cita il nodo e si rifà solo se la
  variabile è cambiata (e si dice quale).
- **E5 PREVISIONE SCRITTA CON IL MODELLO DEL FRAME.** Prima di ogni run, una riga: "prevedo X fps perché ...".
  Il titolo tiene 2 frame in volo: T ≈ (R+L)/2, un taglio di c ms di scheda vale ~c/2 di frame. Leve sotto 3 ms
  si giudicano **in pacchetto**, mai da sole (da sole sono sempre "rumore"). Esito lontano dalla previsione = si
  cerca perché prima della run successiva.
- **E6 UN BLOCCO NON FERMA IL LOOP** (2026-11-27: fermo sui contatori GPU, Host: "non stavi seguendo un goal?").
  Il blocco va in STATE.md (BLOCCO) in una riga e la leva successiva parte nello stesso turno.
- **E7 NOMI ONESTI.** Ogni run si annuncia per quello che è: riscaldamento, sonda (immagine sbagliata ammessa),
  misura. "Verifica" solo per una run che passa E1 ed E2.
- **E8 NIENTE CPU PESANTE DURANTE UNA RUN MISURATA** (build, warm, replay, scansioni pesanti): spostano il lane.
- **E9 CONTROLLO PRIMA DI OGNI AZIONE COSTOSA** (run, bump, merge, commit), una riga ciascuno: (1) cosa sposta di
  G1-G4 e di quanto; (2) exe, set e libreria sono quelli del riferimento?; (3) cosa invaliderebbe il risultato;
  (4) nodo d'archivio pertinente.
- **E10 ERRORE = GUARDIA NUOVA NELLO STESSO TURNO.** Se un errore passa comunque, nello stesso turno si aggiunge la
  guardia che lo avrebbe fermato (script, test o riga qui, con la run che l'ha mostrato). Mai solo "errore mio".
- **E11 IL GIOCO NON SI LANCIA SOTTO UN PROFILER** (avmgx1, 2026-11-27: sotto nsys la guardia non trovava victory.exe e
  il gioco girava senza guardia; con `-t dx12` +4 GB di privata, chiuso a 34 GB; il log resta vuoto). Contatori GPU solo
  device-wide accanto a una run normale (`run.ps1 -GpuMetricsAt <s>`). **Meccanico:** guard.ps1 cerca l'exe fra i
  discendenti e, se non lo trova, chiude l'albero del cmd; il VERDICT richiede report delle pipeline e ≥4 finestre di mondo.

## 3. Protocollo di run (standard)

```
powershell -NoProfile -ExecutionPolicy Bypass -File D:\Tools\victory\warm.ps1 -Set "<set>"
powershell -NoProfile -ExecutionPolicy Bypass -File D:\Tools\victory\run.ps1 -Name <nome> [-Set "VICTORY_X=1,VICTORY_Y=2"] [-CapSec 330] [-WarmUp]
```
- **Pre-flight (run.ps1, meccanico):** rifiuta se l'exe è più vecchio di un `.rs`/`Cargo.toml` o se manca il timbro
  di `warm.ps1` per quell'exe e quel set (`D:\Tools\victory\warm-stamp.txt`, `stamp.ps1`). `-WarmUp` salta il timbro
  e marca la run come riscaldamento. **Post-run:** riga `VERDICT:` (misura valida solo con made=0).
- Lanciare sempre dal tool PowerShell, in background. Dentro c'è la guardia di memoria (`D:\Tools\victory\guard.ps1`,
  MaxPriv 34 GB, MinFreeCommit 10 GB, kill solo per PID verificato).
- Input fisso: Enter ogni 5 s da 15 a 140 s; `VICTORY_KEYS_ONLY=1` (pad e tastiera reali ignorati). La schermata
  iniziale è su a ~12 s. Il personaggio è alla finestra della cella da ~213 s, mondo stabile da ~220-235 s.
  **Autostop** (`D:\Tools\victory\autostop.py`): appena 4 finestre di 5 s di fila mostrano il mondo (≥4000 draw,
  ≥3 fps) la run si chiude da sola (~4 min in tutto); CapSec 300 è solo il tetto. I dati, non l'attesa.
- Movimento solo se serve a un test, con `-Path "<s>;<frame>:<tasti>,..."`. Misura: 30 frame di stick destro (L)
  ≈ 60° di rotazione. W non ha mosso il personaggio a 270 s (causa non ancora nota).
- Output: log `D:\Temp\victory\<nome>.txt`; catture del titolo in `shots_<nome>` (`CAPTURE_WORLD_EVERY=64`; con il
  default 8 ogni frame salvato costa 235 ms e falsa gli fps); desktop in `desk_<nome>` a 60, 150, 205, 220, 235, 250 s.
- Numeri: `python -I D:\Tools\victory\fps.py <log>` stampa, per finestre di 5 s, fps, mediana, frame più lungo,
  lavoro del thread di submit, copie, copia indietro (ms e MB), scheda ferma, draw.
- Riferimento di confronto: l'ultima run su main **col set standard** (STATE.md, CONOSCENZA MISURATA) con
  `VERDICT: MEASUREMENT valid`; fps medi nelle finestre 195-300 s (`fps1.py`), rumore ±0,7. `ref_base` non si cancella.

## 4. Build e strumenti

| Cosa | Come |
|---|---|
| Build diag | `TEMP=TMP=D:/Temp/victory cargo build -q --profile diag -p victory --features diagnostics` → `target\diag\victory.exe` |
| Test | `cargo test -q -p <crate> [--features diagnostics] <filtro>` |
| Precompilazione | `warm.ps1 -Set "<set>"`: `victory shaders <gioco>` (cache DXIL) + `victory warm --used` (le ~2400 pipeline usate, nella cache del driver, chiave = traduzione + driver) + timbro. La libreria `host-pipelines-<chiave>.bin` la riempie la run `-WarmUp` |
| Switch documentati | README del repository, sezione "Status of this fork": ogni `VICTORY_*` con la misura dell'autore |
| Disassembly / decompilazione dell'exe | Ghidra headless via MCP (`pyghidra-mcp`, porta 8000, `D:\Tools\start-ghidra-mcp.cmd`), exe già importato |
| Contatori GPU | `run.ps1 ... -GpuMetricsAt 240` (gpumetrics.ps1: nsys `--gpu-metrics-devices=0`, 10 s, 10 kHz, device-wide, accanto al gioco) → `D:\Temp\victory\<nome>_gm.sqlite`; letture con `D:\Temp\victory\gm2.py`/`gm3.py`/`gm4.py` (medie, fasce per SM attivi, tempo su 1-2 SM) |
| Costo per pipeline | `victory replay <cattura> --no-restore --timing detailed --awake --top 60`: ms, comandi e gruppi per dispatch diretta di ogni pipeline (cattura: `VICTORY_RECORD=D:\Temp\victory\rec,VICTORY_RECORD_AT=240`, ~8-11 GB, si cancella dopo) |
| Timeline CPU+GPU | Nsight Systems 2025.5.2 (`C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.5.2`): thread di submit, code, fence |
| Shader | `dxc` (`C:\VulkanSDK\1.4.363.0\Bin\dxc`) per disassemblare o validare il DXIL tradotto |
| Driver | `drvwatch` registra nel log gli errori del driver (nvlddmkm 153); il primo spegne le draw window |
| PIX (GPU capture) | **PIX 2611.19 installato** (`C:\Program Files\Microsoft PIX\2611.19\`, OK dell'Host 2026-11-26). Cattura di un frame del titolo: run con `-Set "VICTORY_PIX=C:\Program Files\Microsoft PIX\2611.19\WinPixGpuCapturer.dll,VICTORY_PIX_FRAME=2000"` (diag; `VICTORY_PIX_FRAMES` 1-8, `VICTORY_PIX_OUT`, default `%TEMP%\victory_pix\frame.wpix`; la libreria PSO non si usa in quella run). Lettura: `pixtool open-capture f.wpix save-event-list ev.csv --counters=*` (tempo scheda per draw/dispatch), `save-screenshot`, `save-resource`; `WinPix.exe f.wpix` per l'ispezione a mano (shader, stato, risorse per draw). Si usa per: posterizzazione, specchio, nero in ombra, qualunque draw sbagliata = causa vista nella draw, non dedotta |
| PIX (timing) | `pixtool attach <pid> take-new-timing-capture f.wpix --duration=2000`: timeline CPU+GPU di 2 s del processo in corso (thread di submit, code, fence), in alternativa a Nsight |
| RenderDoc | Non installato (`VICTORY_RENDERDOC=<dll>` già supportato in diag, renderdoc.rs) |

## 5. Vincoli (non negoziabili)

- Nessun bypass di licenza, XStore o DRM. Nessun servizio online. `XCL_NETWORK=0`.
- Mai committare dati del gioco, dump, shader estratti, file SDK o chiavi. Gli intermedi derivati dal gioco in
  `D:\Temp\victory` si cancellano dopo l'analisi. Temp solo su D:; `D:\Temp\victory` sotto ~1 GB.
- Git: merge, mai rebase o force-push su rami condivisi. Il main locale = upstream + nostro. Nessuna PR verso
  fxceless (decisione dell'Host, 2026-11-25). Push solo sul fork `hk` (hkmodd/victory-fork), rami `hk/*`;
  il main del fork resta specchio di upstream. Identità dei commit:
  `-c user.email=64160552+hkmodd@users.noreply.github.com -c user.name=hkmodd`, riga `Co-Authored-By`.
- Mai `cargo fmt` (diff enormi contro upstream). Read prima di Edit. Mai sed per scrivere percorsi Windows.
  `python -` da stdin si blocca: usare file + `python -I`.

## 6. Mappa dei fork (remote solo lettura, push = no_push)

| Remote/ramo | Repository | Cosa porta di meglio (da verificare per misura) |
|---|---|---|
| ghosty/main | DoubleGhosty/cool_fox | Misurato qui 2026-11-27: dietro di noi (14,9 vs 18,4 fps, senza capelli). Donatore di pezzi G4/G2. `scripts/play.ps1` = il suo set |
| origin/main | fxceless/victory-fork | Dal 5dbf5ce22 = trustno1 VIctory-NVIDIA (non si fonde più, solo donatore). Prima: switch di velocità misurati (README); 10,3 fps nelle prime stanze con INDIRECT_GATHER |
| uilian/main | uiliannn/VItoriaBr | Base hyphennn + port da DexNom/Belobug: quad read con helper lane (righe su vernice, vetro, pelle), `victory warm`, priorità alle pipeline, formati 189-191, dot4 impacchettati, ambiente da profondità (Belobug), analisi comparative in `docs/` |
| itsbatt/main | Itsbatt/victory-unified | fxceless + grafica Belobug unificate, cap 30 fps, liste GPU in volo limitate a 128 |
| zettrox/main | Zettrox/ztrx | Il più avanti nella storia (prologo, "Gone Fishing", 1h30 senza crash); grafica peggiore |
| dexnom/main | DexNom/VIctory (AMD) | RT funzionante ("Restore ray structure building" bc3cd4b), shader 25.742/25.758, streaming texture |
| belobug/main | Belobug/VIctory-Belobug-AMD | Mondo vicino al retail: luci, vegetazione, acqua, cielo; scheduler delle pipeline, allocatore dei descrittori |
| joshuu/victory-amd | JoshuuDev/victory-amd | Base di "Doomsday" (privato, 78,7%: RT a 3-5 fps, quasi mai crash) |
| zslayer/next | ZSlayerHQ/zslayer-pc-port-runtime | Runtime da cui parte DexNom |
| marvisak/main | Marvisak/VIctory | Upstream originale |

Classifica pubblica: https://portwatch-vi.xyz/ (dati dichiarati dai README; le misure contano solo se riprodotte qui).
I fork AMD vanno verificati su NVIDIA: un pezzo che dipende da wave64 o da comportamenti RDNA si misura prima.

## 7. Conoscenza misurata (ancore; dettagli e date in STATE.md)

- Oggi (2026-11-27, x1 = RT skip + fusione xavi): **20,8 fps** nel mondo col set standard, frame ~48 ms (avmv7b 18,4). Muro doppio:
  **scheda ~55 ms** (liste del titolo 41, nostre 5, buchi 9) e lane ~35 ms. 30 fps = 33 ms di entrambi.
- La scheda è lavoro degli shader tradotti diffuso su ~1270 pipeline (nessuna voce ≥5 ms); l'emulazione wave64 NON
  è il muro (nodo 234); le leve a switch del traduttore sono esaurite.
- Contatori GPU (avmgm2, mondo, 19,5 fps): scheda occupata 97%, ferma solo 1,7%; SM attivi 49%, issue 16%, DRAM 10%.
  **~9 ms/frame su 1-2 SM di 76** (dispatch piccole o seriali) e, a SM pieni, issue 36% con 60% di slot di warp liberi
  (registri). Non è banda né istruzioni pure: le leve sono le dispatch seriali e l'occupazione.
- Storico: main 0b0074e 4,5-5,4 fps con muro = thread di submit 135-190 ms (risolto dalla linea avatarmanz).
- Ombre nere = builder RT spento (`BVH_BUILDER=0`, guardia di memoria).
