# Registro de Continuacion - KAMUSIC

Fecha: 2026-06-17 15:35 CEST

## Estado Actual
- Snap local generado y subido: `kamusic_0.1.34_amd64.snap`.
- Revision Store creada: 31.
- Estado de la revision 31: pendiente de revision manual en Snapcraft.
- Canal `stable` actual: `0.1.33`, revision 30.
- Motivo indicado por Snapcraft: `human review required due to 'deny-connection' constraint (interface attributes)`.
- Intento de release manual: `snapcraft release kamusic 31 stable`.
- Resultado del release manual: `resource-not-ready: Revision 31 is not approved`.
- Metadata de tienda actualizada desde `kamusic_0.1.34_amd64.snap` con `snapcraft upload-metadata --force`.

## Verificaciones Realizadas
- `snapcraft upload kamusic_0.1.34_amd64.snap --release=stable`: subio la revision 31, pero quedo retenida para revision manual.
- `snapcraft revisions kamusic`: confirma `0.1.34` como revision 31 sin canal.
- `snapcraft status kamusic`: confirma `stable` en `0.1.33`, revision 30.
- `cargo check`: correcto, con warnings no bloqueantes existentes sobre `audio_sink`, `video_sink` y `video_paintable`.
- `git diff --check`: correcto despues de retirar un espacio sobrante en `src/app.rs`.

## Puntos de Continuacion
1. Revisar en el dashboard de Snapcraft la revision 31 y aprobar/resolver la revision manual.
2. Cuando este aprobada, publicar con:
   - `snapcraft release kamusic 31 stable`
3. Confirmar despues con:
   - `snapcraft status kamusic`
   - `snap info kamusic`

---

Fecha: 2026-06-11 13:14 CEST

## Estado Actual
- Implementado y publicado el modo compacto del reproductor.
- Version actual publicada en Ubuntu Store: `0.1.33`.
- Snap local generado: `kamusic_0.1.33_amd64.snap`.
- Canal publicado: `stable`.
- Revision Store actual: 30.
- Mensaje de Snapcraft: `Revision 30 created for 'kamusic' and released to 'stable'`.

## Cambios Realizados
1. `src/config/settings.rs`
   - Se anadio `compact_mode` a `Settings`.
   - El estado compacto se guarda y se restaura al abrir la app.

2. `src/ui/window.rs`
   - Se anadio boton de minimizar reproductor junto al boton de cerrar.
   - Se creo una vista compacta en un `gtk::Stack`.
   - La vista compacta muestra portada, titulo de la reproduccion, boton restaurar, play, pausa y selector de playlists/carpetas.
   - La vista compacta es arrastrable desde la portada.
   - El boton restaurar vuelve a la vista completa y recupera el tamano normal guardado.
   - Se evita guardar el tamano compacto como tamano normal de ventana.
   - Se anadio `render_compact_playlists` para listar playlists/carpetas en el menu compacto.
   - Se reutiliza la logica existente de reproduccion para play, pausa y seleccion de playlist desde el modo compacto.
   - Se corrigio que el gesto de arrastre interceptara clics del boton restaurar.
   - Se reemplazaron iconos simbolicos dependientes del tema por assets empaquetados.

3. `data/icons/restore.svg`
   - Nuevo icono local para el boton de restaurar en modo compacto.

4. `data/icons/minimize.svg`
   - Nuevo icono local para el boton de minimizar reproductor.

5. `snap/snapcraft.yaml`
   - Version actualizada primero a `0.1.32` para publicar el modo compacto.
   - Version actualizada despues a `0.1.33` para corregir el icono de restaurar en el snap instalado.

## Verificaciones Realizadas
- `cargo fmt --check`: correcto.
- `cargo check`: correcto.
- `cargo run`: la app compilo y arranco correctamente durante las pruebas locales.
- `snapcraft --destructive-mode`: correcto para `kamusic_0.1.32_amd64.snap`.
- `snapcraft upload --release=stable kamusic_0.1.32_amd64.snap`: correcto, revision 29 publicada en `stable`.
- `snapcraft --destructive-mode`: correcto para `kamusic_0.1.33_amd64.snap`.
- Durante la build `0.1.33`, Snapcraft confirmo la instalacion de:
  - `data/icons/minimize.svg`
  - `data/icons/restore.svg`
- `snapcraft upload --release=stable kamusic_0.1.33_amd64.snap`: correcto, revision 30 publicada en `stable`.

## Avisos Observados
- `cargo check` mantiene warnings no bloqueantes ya existentes:
  - `audio_sink` y `video_sink` en `GstBackend`.
  - `video_paintable` en backend/player.
- Snapcraft mantiene warnings de linters sobre librerias GPU y librerias no usadas. No bloquearon el empaquetado ni la publicacion.
- En snaps instalados, no conviene depender de iconos GTK simbolicos por nombre (`view-restore-symbolic`, `window-minimize-symbolic`, etc.). Se sustituyeron por SVGs locales empaquetados.
- GTK4 no ofrece una API portable para guardar/restaurar coordenadas X/Y exactas de ventana en todos los gestores, especialmente Wayland. El modo compacto queda arrastrable y conserva el estado compacto, pero no persiste una posicion exacta portable.

## Puntos de Continuacion
1. Instalar desde Store y validar que la version `0.1.33` muestra el icono de restaurar:
   - `sudo snap refresh kamusic --stable`
   - o `sudo snap install kamusic --stable`
2. Probar en el snap instalado:
   - Boton minimizar reproductor.
   - Boton restaurar en modo compacto.
   - Play y pausa desde modo compacto.
   - Selector de playlists/carpetas desde modo compacto.
3. Considerar limpiar warnings de `video_paintable` si no se usara la vista de video.
4. Revisar warnings de Snapcraft si se quiere reducir tamano del snap o ajustar soporte GPU mediante content interfaces.

---

Fecha: 2026-06-10 12:55 CEST

## Estado Actual
- La reproduccion continua esta implementada para la cola local/favoritos y para la cola online/radio.
- Al terminar una pista, GStreamer emite EOS, la UI recibe `PlayerEvent::EndOfStream` y reutiliza `play_next_active`.
- Si no hay siguiente elemento, la app detiene el reproductor, marca `is_playing = false`, limpia `current_index` y actualiza MPRIS como detenido.
- Snap generado y publicado en Ubuntu Store:
  - Archivo local: `kamusic_0.1.30_amd64.snap`
  - Tamano: 217 MB
  - Canal: `stable`
  - Revision Store: 27
  - Mensaje de Snapcraft: `Revision 27 created for 'kamusic' and released to 'stable'`

## Cambios Realizados
1. `src/audio/player.rs`
   - Se anadio `PlayerEvent`.
   - Se anadio `Player::new_with_events(Sender<PlayerEvent>)`.
   - Se retiro el constructor sin eventos porque ya no se usa.

2. `src/audio/gst_backend.rs`
   - `GstBackend::new` ahora acepta `Option<Sender<PlayerEvent>>`.
   - El `BusWatchGuard` envia `PlayerEvent::EndOfStream` al recibir `MessageView::Eos`.

3. `src/ui/window.rs`
   - Se crea un canal `mpsc` para eventos del reproductor.
   - La ventana crea el reproductor con `Player::new_with_events`.
   - Se anadio un `glib::timeout_add_local` cada 120 ms para consumir eventos del reproductor en el hilo GTK.
   - EOS llama a `play_next_active`, igual que el boton Siguiente y MPRIS Next.
   - `play_next_active` comprueba si existe siguiente elemento antes de reproducir.
   - Se anadio `finish_playback` para cerrar correctamente el fin de cola.

4. `snap/snapcraft.yaml`
   - Version del snap actualizada de `0.1.29` a `0.1.30`.

## Verificaciones Realizadas
- `cargo fmt --check`: correcto.
- `cargo check`: correcto.
- `snapcraft`: correcto, genero `kamusic_0.1.30_amd64.snap`.
- `snapcraft whoami`: sesion activa como `kampos.info@gmail.com` con permisos de push/release.
- `snapcraft upload --release=stable kamusic_0.1.30_amd64.snap`: correcto, revision 27 publicada en `stable`.

## Avisos Observados
- `cargo check` deja warnings no bloqueantes de codigo no usado:
  - `audio_sink` y `video_sink` en `GstBackend`.
  - `video_paintable` en backend/player.
- Snapcraft dejo warnings de linters sobre librerias GPU y librerias no usadas. No bloquearon el paquete ni la publicacion.
- `snap info kamusic` se quedo colgado al consultar la Store tras publicar. Se termino el proceso colgado; la publicacion ya estaba confirmada por `snapcraft upload`.

## Puntos de Continuacion
1. Probar desde Store en una maquina limpia: `sudo snap install kamusic`.
2. Validar reproduccion continua en:
   - Biblioteca local completa.
   - Carpeta/playlist seleccionada desde la barra lateral.
   - Favoritos.
   - Radio/online, si aplica.
3. Considerar limpiar warnings de `video_paintable` si ya no se usa o reconectarlo si la vista de video sigue siendo necesaria.
4. Revisar los warnings de Snapcraft si se quiere reducir tamano del snap o ajustar soporte GPU con content interfaces.
