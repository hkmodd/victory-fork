# AGENTS.md — VIctory fork (hyphennn base), contratto operativo dell'agente

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

## 2. Il loop (ogni ciclo, nell'ordine; nessuna fase si salta)

1. **ALIGN.** Esegui `bash /d/Tools/victory-align.sh`: fa il merge di origin/main (fxceless) nel main locale e
   ricompila. Gira anche da cron ogni 20 min (va ricreato a ogni sessione: CronCreate `7,27,47 * * * *`).
2. **SCAN.** Esegui `bash /d/Tools/victory-forkscan.sh`: stampa i commit nuovi di ogni fork della famiglia
   (§6) dall'ultima scansione. Cron ogni ora (`41 * * * *`). Ogni commit nuovo viene classificato in
   STATE.md → "Coda porting": G1/G2/G3/G4, il costo, l'evidenza (misura del donatore o sola dichiarazione).
3. **PICK.** Scegli **un** pezzo, quello con il massimo guadagno atteso per unità di lavoro, e con evidenza.
   Priorità: crash/blocco > G1 visibile nella nostra vista > G3 > resto.
4. **PORT / FIX.** Porta il pezzo come commit singolo, con
   `git cherry-pick -x <sha>` (oppure riscrittura manuale se non si applica). Crediti nel messaggio:
   `(port of <Fork> <sha>, GPL-3.0)`. Mai mescolare due pezzi in un commit.
5. **BUILD + TEST.** Build diag (§4). `cargo test` sul crate toccato. Se non compila, il pezzo non entra.
6. **RUN.** Una run standard (§3), una variabile alla volta rispetto al riferimento.
7. **JUDGE.** Si decide sulle immagini affiancate, agli stessi secondi, e sui numeri di `fps.py`.
   Migliora → resta. Uguale ma corretto → resta. Peggiora o dubbio → `git revert`, si annota il perché.
8. **COMMIT + STATE.** Commit locale. STATE.md: FATTO (un nodo), coda aggiornata, PROSSIMO (uno solo).
   Ai checkpoint, push del ramo di integrazione sul fork nostro (`hk`), mai su origin.

**Determinismo.** Stesso input, stessa vista, stessi secondi, stesso intervallo di cattura. Non si dichiara
niente (mondo, fps, fix) senza uno screenshot e un log **di quella run**. Le ipotesi si scrivono con la loro
evidenza (codice, disassembly, log, misura del donatore). "Proviamo" non è un metodo: prima la causa, poi il fix.

## 3. Protocollo di run (standard)

```
powershell -NoProfile -ExecutionPolicy Bypass -File D:\Tools\victory\run.ps1 -Name <nome> [-Set "VICTORY_X=1,VICTORY_Y=2"] [-CapSec 330]
```
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
- Riferimento di confronto: l'ultima run standard su main senza switch (in STATE.md). `ref_base` non si cancella.

## 4. Build e strumenti

| Cosa | Come |
|---|---|
| Build diag | `TEMP=TMP=D:/Temp/victory cargo build -q --profile diag -p victory --features diagnostics` → `target\diag\victory.exe` |
| Test | `cargo test -q -p <crate> [--features diagnostics] <filtro>` |
| Switch documentati | README del repository, sezione "Status of this fork": ogni `VICTORY_*` con la misura dell'autore |
| Disassembly / decompilazione dell'exe | Ghidra headless via MCP (`pyghidra-mcp`, porta 8000, `D:\Tools\start-ghidra-mcp.cmd`), exe già importato |
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
| origin/main | fxceless/victory-fork | **Base.** Switch di velocità misurati (README); 10,3 fps nelle prime stanze con INDIRECT_GATHER |
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

- Mondo su main 0b0074e, default: 4,5-5,4 fps; frame 195-215 ms; copia indietro 125-150 ms / 52 MB; ~7000 draw.
- Set del README (ASYNC_SUBMIT, QUEUE2, INDIRECT_WINDOW, TEXTURE_NO_WAIT, COPY_PLAIN): 5,0-6,4 fps; copia
  indietro 30-40 ms; **muro = thread di submit, 135-190 ms per frame**.
- README upstream (465b94f): ~50 ms di scheda per frame sono le passate del titolo tradotte. Per i 30 fps servono
  sia la CPU (submit, copie) sia shader tradotti più snelli.
- Ombre nere = ray tracing spento nel nostro albero.
