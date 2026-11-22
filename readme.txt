======================================================================
 NARANJAHUB v1.4.1 — ADMINISTRADOR DE APPIMAGES
 Creado por Leyter Vázquez. V
 Licencia: GNU GPL v3 (ver sección 10)
======================================================================

ÍNDICE
 1. ¿Qué es NaranjaHub?
 2. Compatibilidad y requisitos
 3. Entregables y cómo usarlos
 4. Funciones completas
 5. Guía pestaña por pestaña
 6. Archivos y carpetas que usa la aplicación
 7. Preguntas frecuentes y solución de problemas
 8. Registro de cambios (changelog)
 9. Hoja de ruta de próximas versiones
10. Licencia GNU GPL v3
11. English summary

----------------------------------------------------------------------
1. ¿QUÉ ES NARANJAHUB?
----------------------------------------------------------------------

NaranjaHub es un administrador de aplicaciones AppImage para Linux,
simple por fuera y completo por dentro. Te permite reunir todas tus
AppImage en una sola carpeta, integrarlas en el menú de aplicaciones de
tu escritorio, lanzarlas, revisar si hay versiones nuevas publicadas en
GitHub y eliminarlas de forma segura.

Está programado 100% en Rust (interfaz egui), no necesita FUSE, no
necesita Python, Node ni ningún tiempo de ejecución externo, y funciona
igual en GNOME, KDE Plasma, COSMIC y XFCE (X11 y Wayland).

Características visuales: tema naranja y negro, esquinas suaves, logo
propio en la interfaz, ventana oscura con icono propio, textos en
español (por defecto) o inglés seleccionables desde Ajustes.

Novedades de la versión 1.4.1: corregido el bug más molesto de la
pestaña Actualizaciones: al comprobar actualizaciones SIN conexión a
internet la ficha se quedaba bloqueada eternamente en "Comprobando...".
Ahora la comprobación caduca sola en segundos con el mensaje claro
"sin conexión a internet", y un VIGILANTE de trabajos aborta cualquier
operación bloqueada (45 s en comprobaciones, 120 s sin progreso en
copias y descargas). Además, las descargas muestran "Descargando..."
con su progreso, los errores de red se explican en claro y "Analizar"
ya no crea accesos directos huecos si borraste la AppImage fuera de
la app. Detalles en la sección 8.

Novedades de la versión 1.4.0: en la pestaña Integrar, el botón
"Explorar archivos" ya no cae sobre la opción "Mover en lugar de
copiar" en ventanas estrechas (opciones y botón tienen ahora filas
propias) y el archivo marcado se puede DESMARCAR cuando quieras
(botón "Desmarcar" junto a "Integrar", o pulsando de nuevo sobre
él en el explorador). En la pestaña Actualizaciones puedes definir
a mano la DIRECCIÓN DE ACTUALIZACIÓN de cada AppImage (URL de GitHub
tipo https://github.com/usuario/repo, usuario/repo o la cadena
gh-releases-zsync completa); queda guardada en disco y tiene
prioridad sobre la información embebida en el archivo. Detalles en
la sección 8.

----------------------------------------------------------------------
2. COMPATIBILIDAD Y REQUISITOS
----------------------------------------------------------------------

 - Arquitectura: x86_64 (64 bits).
 - Escritorios probados por diseño: GNOME, KDE Plasma, COSMIC (System76)
   y XFCE. También funciona en MATE, Cinnamon, LXQt y otros que sigan
   las especificaciones freedesktop.
 - Sesiones gráficas: X11 y Wayland. Por defecto NaranjaHub arranca
   en X11/XWayland (motor con arrastrar y soltar funcional) y cae a
   Wayland nativo si XWayland no está disponible; se puede fijar en
   Ajustes > Motor gráfico o con la variable NARANJAHUB_BACKEND.
 - Único requisito en tiempo de ejecución: el stack gráfico de tu
   escritorio (libX11/libwayland/libGL), que cualquier sistema con
   escritorio ya tiene instalado.
 - No requiere: FUSE (usa --appimage-extract interno), librerías GTK/Qt
   externas, Python, Node.js, Java ni conexión permanente a Internet
   (solo se conecta para comprobar/descargar actualizaciones).

----------------------------------------------------------------------
3. ENTREGABLES Y CÓMO USARLOS
----------------------------------------------------------------------

Los tres formatos son portables: la aplicación no se apoya en ninguna
otra app ni dependencia externa.

3.1 NaranjaHub-1.4.1-x86_64.AppImage
    - Descarga, dale permiso de ejecución y ábrelo:
        chmod +x NaranjaHub-1.4.1-x86_64.AppImage
        ./NaranjaHub-1.4.1-x86_64.AppImage
    - También puedes abrirlo con doble clic desde tu gestor de archivos.
    - Si tu sistema no tiene FUSE, ejecútalo así:
        ./NaranjaHub-1.4.1-x86_64.AppImage --appimage-extract-and-run
    - Truco: integra la propia AppImage de NaranjaHub desde su pestaña
      Integrar para tenerla en el menú.

3.2 naranjahub_1.4.1_amd64.deb
    - Paquete Debian/Ubuntu/Mint/Pop. Instala en el sistema:
        sudo dpkg -i naranjahub_1.4.1_amd64.deb
      o
        sudo apt install ./naranjahub_1.4.1_amd64.deb
    - No declara dependencias: no tocará tus paquetes.
    - Instala en /usr/bin/naranjahub, con icono y entrada de menú.

3.3 NaranjaHub-1.4.1-x86_64.run
    - Instalador portable autoextraíble, no necesita root:
        ./NaranjaHub-1.4.1-x86_64.run
      (instala en ~/.local/opt/NaranjaHub y crea la entrada de menú)
    - Opciones:
        --dir /ruta/de/destino   Instala donde quieras
        --portable               Extrae junto al archivo (ideal para USB)
        --no-menu                Sin acceso directo en el menú
        --uninstall              Desinstala limpiamente
        --help                   Ayuda
    - Solo requiere sh, tar y base64, que todo Linux trae incluidos.

3.4 NaranjaHub-1.4.1-codigo-fuente.zip
    - Código fuente completo (Rust), iconos, plantillas de empaquetado,
      licencia y scripts para regenerar TODOS los entregables.
    - Para compilarlo: rustup + cargo (https://rustup.rs)
        unzip NaranjaHub-1.4.1-codigo-fuente.zip
        cd naranjahub
        cargo build --release
        # binario en target/release/naranjahub

3.5 readme.txt
    - Este archivo. Se incluye también dentro del .deb y del .run.
    - La sección 8 (changelog) se actualizará con cada versión.

3.6 SHA256SUMS.txt
    - Sumas de verificación para comprobar la integridad de los
      entregables:
        sha256sum -c SHA256SUMS.txt

----------------------------------------------------------------------
4. FUNCIONES COMPLETAS
----------------------------------------------------------------------

GESTIÓN (pestaña Biblioteca)
 - Escaneo automático de la carpeta gestionada (por defecto
   ~/Applications) con detección de archivos .AppImage/.appimage.
 - Búsqueda instantánea por nombre y orden por Nombre, Fecha o Tamaño.
 - Lanzamiento de aplicaciones con un clic (da permiso de ejecución si
   no lo tenía).
 - Fichas de cada app: icono, nombre limpio (sin sufijos -x86_64,
   _amd64, etc.), tamaño, fecha de modificación y versión si la AppImage
   la declara.
 - Insignias de estado: Integrada / Sin menú / Sin permiso de ejecución.
 - Quitar del menú de aplicaciones sin borrar el archivo (limpia
   también sus iconos instalados).
 - Mostrar la AppImage en tu gestor de archivos.
 - Enviar a la papelera freedesktop (con confirmación configurable):
   el archivo queda recuperable desde la papelera del sistema de tu
   escritorio (respeta $XDG_DATA_HOME) y se quita también del menú.
 - Abrir la carpeta gestionada y recargar la biblioteca.
 - La biblioteca se refresca sola al terminar cada integración,
   análisis o actualización: los cambios se ven al instante.

INTEGRACIÓN (pestaña Integrar)
 - Arrastrar y soltar uno o varios .AppImage sobre la ventana; cada
   archivo soltado se registra en el Registro de actividad. Funciona
   en X11 y en Wayland (la app arranca en X11/XWayland, el motor con
   soporte de soltar archivos; en Ajustes puedes usar Wayland nativo
   si tu sistema no tiene XWayland).
 - Nueva integración por RUTA PEGADA: pega (Ctrl+V) una ruta normal,
   con «~» o una URI file:// (una por línea) y pulsa "Integrar ruta";
   útil si arrastrar no está disponible en tu escritorio.
 - Navegador de archivos integrado (sin diálogos externos: mismo
   comportamiento en todos los escritorios).
 - El botón Integrar queda siempre visible encima del explorador y toda
   la pestaña tiene scroll, por lo que ningún botón se esconde en
   ventanas pequeñas. Las opciones (crear acceso directo / mover) y el
   botón "Explorar archivos" están en filas separadas: nunca se
   superponen aunque la ventana sea estrecha.
 - DESMARCAR el archivo elegido (nuevo en 1.4.0): el archivo marcado
   en el explorador ya no queda atrapado; se suelta con el botón
   "Desmarcar" junto a "Integrar" (visible también con el explorador
   cerrado) o pulsando de nuevo sobre él en el explorador.
 - Copia (o traslado) a la carpeta gestionada con barra de progreso.
 - Permiso de ejecución automático (chmod 755).
 - Extracción del icono interno de la AppImage (a través de
   --appimage-extract, sin FUSE) y del nombre, comentario, categorías y
   versión desde su .desktop interno.
 - El icono se instala en el TEMA de iconos del usuario
   (~/.local/share/icons/hicolor, PNG por tamaño o SVG escalable) y el
   acceso directo lo referencia por nombre (Icon=slug): así se ve bien
   en el menú de GNOME, KDE Plasma, COSMIC y XFCE. Si la AppImage no
   trae icono usable se usa el genérico de NaranjaHub.
 - La Biblioteca muestra el icono real de cada AppImage integrada.
 - Creación de acceso directo .desktop en
   ~/.local/share/applications (compatible GNOME/KDE/COSMIC/XFCE) con
   el nombre oficial de la app y refresco automático de la caché de
   aplicaciones del escritorio (update-desktop-database si existe).
 - Reintegrar la misma AppImage no crea accesos duplicados: se
   actualiza el existente.
 - Opciones: mover en lugar de copiar (ahora por renombrado
   instantáneo cuando es posible, sin dejar duplicados si algo falla)
   y crear o no el acceso directo.
 - Nombres únicos automáticos para evitar sobrescribir ("App (2)").

REGISTRO (pestaña Registro) — nuevo en 1.1.0
 - Historial detallado de TODO lo que hace la aplicación, con fecha y
   hora locales, nivel (Info, OK, Aviso, Error) y categoría
   (Aplicación, Biblioteca, Integración, Menú de aplicaciones, Papelera,
   Ajustes, Actualizaciones, Arrastrar y soltar, Informe, Lanzamiento).
 - Quedan registrados, entre otros: cada integración (origen,
   destino, tiempo y si se creó el acceso directo), cada lanzamiento,
   cada borrado a papelera, los cambios de ajustes e idioma, las
   comprobaciones y descargas de actualizaciones, los archivos
   arrastrados e ignorados y todos los fallos con su error exacto.
 - Filtro "Todo / Fallos y avisos", contador de eventos y limpieza del
   registro con un clic.
 - El registro se guarda en disco
   (~/.local/share/NaranjaHub/logs/actividad.log) y se recarga al
   volver a abrir la aplicación; rota solo cuando pasa de 512 KB
   (copia de seguridad en actividad.log.old).
 - EXPORTACIÓN DE INFORME DE FALLOS Y ERRORES: botón "Exportar informe
   de fallos" que genera un archivo de texto (informe-naranjahub-
   AAAAMMDD-HHMMSS.txt) en la carpeta que elijas con el navegador
   integrado. El informe incluye: resumen (eventos/avisos/errores),
   entorno (sistema operativo, escritorio, sesión, usuario, rutas),
   la lista completa de fallos y avisos registrados y el registro
   completo de actividad. Ideal para enviar el detalle de un problema.

ACTUALIZACIONES (pestaña Actualizaciones)
 - DIRECCIÓN DE ACTUALIZACIÓN MANUAL (nuevo en 1.4.0): si una AppImage
   no declara información (o la tiene mal), pulsa "Definir dirección de
   actualización" en su ficha y escribe la URL del repositorio de
   GitHub (https://github.com/usuario/repo), solo usuario/repo o la
   cadena completa gh-releases-zsync|usuario|repo|etiqueta|patrón. Si
   no escribes el patrón se deduce del nombre del archivo. La dirección
   se guarda en disco (~/.local/share/NaranjaHub/updates.json), tiene
   prioridad sobre la embebida, se puede editar o quitar, sigue a la
   app si una actualización cambia el nombre del archivo y, al
   guardarla, la comprobación se lanza de inmediato.
 - La comprobación NUNCA se queda colgada (nuevo en 1.4.1): el agente
   de comprobaciones caduca la petición entera en 30 s y un vigilante
   de trabajos aborta cualquier operación bloqueada (45 s en
   comprobaciones; 120 s sin progreso en copias y descargas). Sin
   internet verás "Error: sin conexión a internet (...)" o "tiempo
   agotado al comprobar (¿hay conexión a internet?)" en la ficha y en
   el Registro; con red, la ficha pasa de "Comprobando..." a "Al día",
   "Nueva versión: ..." o "Descargando..." con barra de progreso.
 - Detecta el metadato gh-releases-zsync (owner|repo|patrón) leyendo
   la sección ELF .upd_info de la AppImage (la fuente canónica que
   escribe appimagetool con --updateinformation) y, como respaldo, el
   .desktop interno. Antes solo se miraba el .desktop y la mayoría de
   las AppImage reales no traían la información: por eso ahora sí se
   encuentra en apps distribuidas en GitHub.
 - Comando naranjahub --inspect RUTA.AppImage para ver por consola
   qué nombre, icono e información de actualización se detectan.
 - Consulta la última versión en GitHub Releases y compara VERSIONES
   por número (1.2.10 > 1.2.9), no solo por nombre de archivo: si la
   versión publicada no es superior a la instalada se muestra "Al
   día", aunque el nombre local sea distinto (p. ej. "App (2)").
   Ignora blockmaps, firmas y sumas de control.
 - "Comprobar todas" analiza automáticamente las apps que aún no
   declaran información, guarda lo que encuentre en su acceso directo
   y lanza la comprobación sola (encadenado analizar → comprobar).
 - Descarga con barra de progreso y reemplazo seguro del archivo:
   si el asset publicado se llama igual que el archivo local se
   sustituye EN SITIO (respaldo previo; si la instalación falla se
   restaura la versión anterior intacta) y si cambia el nombre se
   añade la nueva versión y se limpian archivo y acceso del viejo.
   La descarga temporal usa sufijo .part para no asomar en la
   biblioteca a medias.
 - Botón Analizar para leer los metadatos de una AppImage integrada
   antes.
 - Comprobación de todas las apps de golpe, y opcional al iniciar.
 - Los errores (sin red, límite de GitHub, sin patrón) se muestran
   claros, no bloquean la app y quedan en el Registro.

AJUSTES (pestaña Ajustes)
 - Idioma: Español (por defecto) e Inglés; el cambio es inmediato y
   también afecta al Registro de actividad y a sus mensajes.
 - Carpeta de aplicaciones personalizable con el navegador integrado,
   botón "Predeterminada" y "Abrir carpeta".
 - Motor gráfico (backend): Automático (recomendado; X11/XWayland
   primero para que arrastrar y soltar funcione, Wayland nativo de
   respaldo), X11/XWayland o Wayland nativo. El cambio se aplica al
   reiniciar y queda registrado.
 - Comprobar actualizaciones al iniciar (sí/no).
 - Confirmar antes de enviar a la papelera (sí/no).
 - Tarjeta del Registro de actividad: ruta del archivo y botón para
   abrir su carpeta.
 - Restablecer todos los ajustes a los valores por defecto.
 - Los ajustes se guardan al instante en el archivo de configuración.

EXTRA
 - Modo consola sin interfaz:
     naranjahub --version        Muestra versión y autoría
     naranjahub --help           Ayuda
     naranjahub --selftest       25 simulaciones internas de la lógica
                                 (útil para verificar una instalación)
     naranjahub --inspect RUTA   Metadatos detectados de una AppImage
                                 (nombre, icono, información de
                                 actualización y a qué repo apunta)
 - Variable de entorno NARANJAHUB_BACKEND=x11|wayland|auto para
   forzar el motor gráfico (anula lo guardado en Ajustes).
 - Notificaciones tipo "toast" para cada operación.
 - Tema exclusivo naranja/negro con logo propio en la barra lateral y
   en la pestaña Acerca de.
 - Código revisado: sin código muerto, sin avisos del compilador ni de
   clippy; cálculo de la hora local corregido (respeta el horario de
   verano de tu zona leyendo /etc/localtime o TZ).

----------------------------------------------------------------------
5. GUÍA PESTAÑA POR PESTAÑA
----------------------------------------------------------------------

BIBLIOTECA: lista de tus AppImage integradas o copiadas. Usa la caja de
búsqueda, el selector "Orden:" y los botones Actualizar y Abrir carpeta.
En cada ficha tienes Lanzar y Opciones (integrar/quitar del menú,
mostrar en carpeta, enviar a la papelera).

INTEGRAR: arrastra archivos .AppImage a la zona naranja, pega una ruta
en el recuadro "pegar ruta" y pulsa Integrar ruta, o pulsa "Explorar
archivos" para navegar con el navegador interno, selecciona uno y pulsa
Integrar (el botón aparece arriba, junto al nombre del archivo
seleccionado, y nunca se esconde). ¿Te equivocaste de archivo? Pulsa
"Desmarcar" junto a Integrar (o vuelve a pulsar el archivo en el
explorador) y elige otro. Marca "Mover en lugar de copiar" si no
quieres conservar el original.

ACTUALIZACIONES: pulsa "Comprobar todas" o el botón Comprobar de cada
app. Si aparece "Nueva versión", pulsa Actualizar para descargarla e
instalarla automáticamente en tu carpeta. Si una app no trae
información de actualización, usa "Definir dirección de actualización"
en su ficha para indicarle el repositorio de GitHub; ahí mismo puedes
editarla o quitarla con "Editar dirección" y "Quitar dirección".

REGISTRO: todo el historial de la app con fecha y hora. Filtra con
"Todo / Fallos y avisos", revisa cada evento con su nivel y categoría,
limpia el registro si quieres empezar de cero y usa "Exportar informe
de fallos" para guardar el informe completo (con entorno, fallos y
registro completo) donde quieras.

AJUSTES: cambia el idioma (Español/English), la carpeta de
aplicaciones, el motor gráfico (Automático/X11/Wayland) y el
comportamiento. Todo se guarda al instante.

ACERCA DE: logo de la app, versión, autoría ("Creado por Leyter
Vázquez. V"), aviso completo de la licencia GNU GPL v3 y enlace al
texto oficial.

----------------------------------------------------------------------
6. ARCHIVOS Y CARPETAS QUE USA LA APLICACIÓN
----------------------------------------------------------------------

 - Carpeta de tus AppImage:      ~/Applications (configurable)
 - Configuración:                ~/.config/NaranjaHub/config.json
 - Iconos extraídos:             ~/.local/share/NaranjaHub/icons/
 - Iconos en el tema:            ~/.local/share/icons/hicolor/
                                 (…/<tamaño>x<tamaño>/apps y scalable/apps)
 - Registro de actividad:        ~/.local/share/NaranjaHub/logs/actividad.log
                                 (respaldo rotado: actividad.log.old)
 - Direcciones de actualización  ~/.local/share/NaranjaHub/updates.json
   manuales (una por AppImage)
 - Accesos directos creados:     ~/.local/share/applications/naranjahub-*.desktop
 - Papelera (borrar):            $XDG_DATA_HOME/Trash o
                                 ~/.local/share/Trash/
 - Archivos temporales:          /tmp/naranjahub-meta-*

Los accesos directos usan las claves X-NaranjaHub-Source,
X-NaranjaHub-UpdateInformation, X-NaranjaHub-OriginalName y
X-NaranjaHub-Version para mantener la biblioteca sincronizada con el
disco; puedes borrarlos sin romper nada, NaranjaHub los regenera.

----------------------------------------------------------------------
7. PREGUNTAS FRECUENTES Y SOLUCIÓN DE PROBLEMAS
----------------------------------------------------------------------

¿Necesito FUSE?                  No. NaranjaHub nunca monta AppImage;
                                 usa su mecanismo interno de extracción.
¿Funciona en Wayland?            Sí (y en X11). Por defecto la app usa
                                 X11/XWayland porque es el motor con
                                 arrastrar y soltar funcional; si no
                                 hay XWayland arranca en Wayland
                                 nativo. Se cambia en Ajustes > Motor
                                 gráfico o con NARANJAHUB_BACKEND.
No aparece en el menú de mi      Cierra y abre la sesión. NaranjaHub ya
escritorio.                      ejecuta update-desktop-database si está
                                 instalado; en la mayoría de escritorios
                                 el icono aparece al momento.
¿Cómo reporto un fallo?          Pestaña Registro > "Exportar informe de
                                 fallos" y envía el archivo generado
                                 junto a la descripción del problema.
¿Dónde queda el informe?         En la carpeta que elijas, con el nombre
                                 informe-naranjahub-FECHA-HORA.txt.
¿El registro ocupa espacio?      Se limita a los últimos 600 eventos en
                                 memoria y a 512 KB en disco (rota solo).
Arrastrar un archivo no hace     Verifica que la extensión sea exactamente
nada.                            .AppImage; los otros archivos se ignoran
                                 y quedan anotados en el Registro. Si el
                                 archivo ya estaba en la biblioteca y con
                                 menú, no se vuelve a integrar. En Wayland
                                 la app usa X11/XWayland (motor con
                                 arrastre); si aun así tu escritorio no
                                 entrega el archivo, pega la ruta en el
                                 recuadro de la pestaña Integrar.
Marca "nueva versión" para       Ya no: la comparación es por NÚMERO de
la misma versión que tengo.      versión desde 1.3.0. Si te pasa con una
                                 app antigua integrada, pulsa Analizar
                                 para regenerar su acceso directo.
Una app no trae información de   Usa "Definir dirección de
actualización y no se puede      actualización" en su ficha (pestaña
comprobar.                       Actualizaciones): acepta la URL del
                                 repositorio de GitHub, usuario/repo o
                                 la cadena gh-releases-zsync completa;
                                 se guarda y se puede editar o quitar.
El explorador marcó un archivo   Pulsa "Desmarcar" junto al botón
y ya no lo quiero integrar.      Integrar, o vuelve a pulsar el archivo
                                 en el explorador: la selección se
                                 limpia al instante.
El icono no sale en el menú.     1.2.0 instala el icono en el tema
                                 hicolor y lo referencia por nombre;
                                 si seguías con un acceso antiguo,
                                 quítalo del menú (Opciones) y vuelve a
                                 integrar la app para regenerarlo.
Una AppImage no tiene icono.     Algunas no incluyen .DirIcon; se usa el
                                 icono genérico de NaranjaHub. Nada falla.
¿Cómo veo qué metadatos tiene    Por consola: naranjahub --inspect
una AppImage?                    /ruta/app.AppImage
Se queda en "Comprobando..."    Corregido en 1.4.1: la comprobación
y nunca termina.                 caduca sola en 30 s y un vigilante aborta
                                 cualquier trabajo a los 45 s. Sin conexión
                                 verás "Error: sin conexión a internet"
                                 o "tiempo agotado al comprobar". Si te
                                 pasara con una versión anterior, actualiza.
"GitHub devolvió 403".           Límite temporal de peticiones de GitHub;
                                 inténtalo de nuevo en unos minutos.
¿Puedo cambiar la carpeta?       Sí, en Ajustes > Carpeta de aplicaciones.
¿Cómo borro una app?             Opciones > Enviar a la papelera (queda
                                 en la papelera del sistema).
El binario no abre en un sistema Con glibc anterior a 2.31 puede no
muy viejo.                       abrir; compila el código fuente allí.

----------------------------------------------------------------------
8. REGISTRO DE CAMBIOS (CHANGELOG)
----------------------------------------------------------------------

Esta sección se actualizará con cada versión. Formato: versión — fecha —
novedades / correcciones.

v1.4.1 — 21 de noviembre de 2026
 + Corregido (IMPORTANTE, pestaña Actualizaciones): al pulsar
   "Comprobar" o "Comprobar todas" SIN conexión a internet, la ficha
   se quedaba bloqueada eternamente en "Comprobando...": la resolución
   DNS y la conexión pueden colgarse mucho más que los tiempos de
   espera del agente de red. Ahora hay TRES capas de protección:
     1) El agente de comprobaciones caduca la petición entera en 30 s
        (conexión en 8 s): sin internet, la comprobación termina sola
        en segundos con "Error: sin conexión a internet (...)".
     2) VIGILANTE DE TRABAJOS en la interfaz: toda comprobación que
        siga sin terminar a los 45 s se aborta con un mensaje claro
        ("tiempo agotado al comprobar (¿hay conexión a internet?)") y
        queda anotada en el Registro de actividad. Ningún trabajo se
        queda colgado para siempre.
     3) Todos los trabajos (integrar, analizar, comprobar, descargar)
        van ahora dentro de un catch_unwind: si algo inesperado entra
        en pánico, la ficha muestra el error en vez de congelarse.
 + Mensajes de red claros: los fallos de DNS, conexión o E/S se
   muestran como "Error: sin conexión a internet (...)" en la ficha,
   el aviso y el Registro, en lugar de un texto técnico críptico.
 + La ficha muestra "Descargando..." (con barra de progreso) durante
   una descarga de actualización; antes decía "Comprobando...",
   confuso.
 + Copias y descargas se detienen solas si llevan 120 s sin avanzar
   nada (disco o red muertos), con error claro; una copia grande con
   progreso activo sigue funcionando con normalidad.
 + Corregido: pulsar "Analizar" sobre una AppImage borrada desde el
   explorador ya NO crea un acceso directo hueco; da error y lo anota
   en el Registro.
 + Revisión completa del código: cero avisos del compilador y de
   clippy; 46 pruebas unitarias y 28 simulaciones en --selftest
   (nuevas: vigilante de trabajos, errores de red claros y AppImage
   borrada), más 2 pruebas reales de red (cargo test -- --ignored)
   que verifican en vivo la comprobación contra GitHub y la caducidad
   sin conexión (IP no enrutable).

v1.4.0 — 21 de noviembre de 2026
 + Corregido (superposición en la pestaña Integrar): el botón
   "Explorar archivos" se dibujaba en la MISMA línea que las opciones
   "Crear acceso directo en el menú de aplicaciones" y "Mover en lugar
   de copiar (borra el original)", y en ventanas estrechas caía justo
   encima de la opción de mover. Ahora las opciones van en su propia
   fila (con salto de línea automático si hace falta) y el botón del
   explorador tiene su propia línea, alineado a la derecha: nunca se
   montan unos encima de otros.
 + Nuevo (desmarcar el archivo elegido): al marcar una AppImage en el
   explorador esta quedaba seleccionada para integrar incluso cerrando
   el explorador, y no había forma de soltarla. Ahora hay un botón
   "Desmarcar" junto a "Integrar" (visible siempre, también con el
   explorador cerrado) y pulsar de nuevo el archivo marcado en el
   explorador lo desmarca; la acción queda anotada en el Registro de
   actividad.
 + Nuevo (pestaña Actualizaciones): DIRECCIÓN DE ACTUALIZACIÓN MANUAL
   por AppImage. En cada ficha hay un botón "Definir dirección de
   actualización" que abre un editor: acepta la URL del repositorio de
   GitHub (https://github.com/usuario/repo, con o sin segmentos extra
   como /releases), solo "usuario/repo", o la cadena completa
   gh-releases-zsync|usuario|repo|etiqueta|patrón. Si no se escribe el
   patrón se deduce del nombre del archivo local: el bloque de versión
   se sustituye por * ("MiApp-1.2.0-x86_64.AppImage" →
   "MiApp-*-x86_64.AppImage"; la arquitectura x86_64 no se confunde
   con una versión).
 + La dirección manual se guarda en
   ~/.local/share/NaranjaHub/updates.json (escritura atómica, JSON
   estable), tiene PRIORIDAD sobre la información embebida en la
   AppImage, se puede EDITAR o QUITAR ("Editar dirección" / "Quitar
   dirección"), queda en el Registro de actividad, se limpia si envías
   la app a la papelera y SIGUE a la AppImage si una actualización
   cambia el nombre del archivo. Al guardar una dirección válida se
   lanza la comprobación de inmediato.
 + Revisión a fondo y limpieza: cero avisos del compilador, de clippy
   y de rustdoc; 42 pruebas unitarias (nuevas para el patrón deducido
   y las direcciones manuales) y 25 simulaciones en --selftest
   (dirección manual en sus tres formatos y almacén persistente
   updates.json).

v1.3.0 — 21 de noviembre de 2026
 + Corregido (arrastrar y soltar en Wayland): la pestaña Integrar no
   recibía nunca los archivos arrastrados en sesiones Wayland porque
   el motor de ventanas winit no implementa la recepción de arrastre
   en Wayland (solo en X11). Ahora NaranjaHub elige su motor gráfico
   al arrancar: modo Automático (por defecto) prueba primero X11/
   XWayland —que sí soporta soltar archivos y sirve igual en GNOME,
   KDE Plasma, COSMIC y XFCE con sesión Wayland— y cae a Wayland
   nativo si XWayland no está disponible, para que la app nunca deje
   de abrir. Nuevo selector en Ajustes > Motor gráfico (Automático /
   X11/XWayland / Wayland nativo, se aplica al reiniciar) y variable
   de entorno NARANJAHUB_BACKEND. El motor elegido queda registrado en
   el Registro de actividad al iniciar.
 + Nueva forma de integrar sin arrastrar: recuadro "pegar ruta" en la
   pestaña Integrar. Acepta rutas normales, con «~» y URI file:// de
   una por línea (con %20 y acentos decodificados), tal como las copia
   un explorador de archivos; también varias de golpe.
 + Corregido (actualizaciones: no encontraba información): la
   información de actualización se buscaba SOLO en el .desktop interno
   de la AppImage, pero appimagetool la escribe en la sección ELF
   .upd_info del ejecutable (así la lee AppImageUpdate). Ahora se lee
   directamente esa sección (parser ELF propio, 32 y 64 bits, sin
   herramientas externas), incluso si la extracción interna falla, y
   el .desktop interno queda como respaldo. Verificado contra una
   AppImage real generada con appimagetool --updateinformation.
 + Corregido (comparación de versiones): se comparaba solo el nombre
   del archivo, así que la misma versión con otro nombre local ("App
   (2).AppImage", un asset de otra arquitectura) se anunciaba como
   "nueva versión" y se volvía a descargar. Ahora se comparan las
   versiones por número (1.2.10 > 1.2.9, 1.2 > 1.1.9, 1.0 = 1.0.0)
   extraídas del nombre del asset y de la versión declarada por la
   app; solo si ninguna versión es comparable se usa el nombre.
 + Reemplazo de actualizaciones a prueba de fallos: la instalación se
   hizo función propia y simulable (apply_update). Con el mismo nombre
   de asset se sustituye EN SITIO con respaldo previo (el acceso
   directo del menú sigue siendo válido); si la instalación falla se
   deshace todo y la versión anterior queda intacta. Con nombre
   distinto se añade el nuevo archivo, se re-integra y se limpian el
   archivo y el acceso directo del viejo. La descarga temporal usa
   sufijo .part para no aparecer a medias en la biblioteca.
 + "Comprobar todas" ahora encadena: las apps que aún no declaran
   información de actualización se analizan (se lee .upd_info y se
   guarda en su acceso directo) y en cuanto la encuentran se lanza su
   comprobación sola. Ya no hace falta analizar app por app.
 + Nuevo comando naranjahub --inspect RUTA.AppImage: muestra por
   consola el nombre, versión, icono, categorías y la información de
   actualización detectada (y el repo/patrón de GitHub al que
   apuntaría), para diagnosticar por qué una app no se puede comprobar.
 + La opción "Mover en lugar de copiar" ahora renombra (instantáneo,
   sin dejar duplicados si el borrado falla) y solo copia+borra entre
   sistemas de archivos distintos.
 + Ajustes: nueva tarjeta "Motor gráfico (backend)" con explicación;
   el cambio queda en el Registro y se aplica al reiniciar.
 + Nuevo aviso en la pestaña Integrar cuando el motor activo es
   Wayland nativo, con las alternativas (explorador o ruta pegada).
 + Limpieza de código muerto: eliminados README.txt antiguo (v1.0.0),
   registro de verificación obsoleto y scripts duplicados de la etapa
   1.0; el paquete .deb toma su versión de Cargo.toml (sustitución
   @VERSION@, ya no puede quedarse desfasado); el zip de código fuente
   incluye los scripts para regenerar TODOS los entregables. Cero
   avisos del compilador y de clippy.
 + Simulaciones ampliadas: 23 en --selftest (lectura de .upd_info en
   ELF con parseo completo, AppImage ELF realista que deja la
   información en el acceso directo, comparación de versiones,
   instalación de actualización con respaldo/restauración e integración
   por ruta pegada) y 39 pruebas unitarias.

v1.2.0 — 21 de noviembre de 2026
 + Corregido (iconos en el menú de aplicaciones): el icono extraído de
   la AppImage nunca llegaba a instalarse porque la carpeta temporal de
   extracción se borraba antes de copiarlo. Ahora el icono se guarda en
   memoria al extraerlo y se instala en el tema de iconos del usuario
   (~/.local/share/icons/hicolor): PNG en su carpeta de tamaño y SVG en
   scalable. El acceso directo lo referencia por NOMBRE (Icon=slug),
   el formato estándar que GNOME, KDE Plasma, COSMIC y XFCE resuelven
   con su cache de iconos; si no se pudo instalar, se usa la ruta
   absoluta como respaldo.
 + El acceso directo usa el nombre oficial de la app (Name= del
   .desktop interno de la AppImage) y se genera siempre a través de la
   misma función, sin duplicados, también al analizar o actualizar.
 + La Biblioteca muestra el icono real de cada AppImage integrada
   (antes todas usaban el icono genérico).
 + Corregido/robustecido (papelera): "Enviar a la papelera" respeta
   $XDG_DATA_HOME (como la papelera de tu escritorio), escribe la
   fecha de borrado en hora local según la especificación freedesktop,
   guarda la ruta original en el .trashinfo y avisa si el archivo ya
   no existe. Quitar del menú o enviar a la papelera limpia también
   los iconos instalados en hicolor y la copia interna.
 + Arrastrar una AppImage que ya está integrada avisa con una
   notificación en lugar de quedarse callado.
 + Nuevas simulaciones en --selftest (ahora 19): AppImage simulada con
   .DirIcon como enlace (icono y nombre correctos en el acceso
   directo), limpieza de iconos al quitar del menú, papelera con
   $XDG_DATA_HOME y tamaños del tema hicolor; 30 pruebas unitarias.
 + Limpieza de código: cero avisos del compilador y de clippy.

v1.1.0 — 21 de noviembre de 2026
 + Pestaña Registro: actividad detallada de todo con fecha y hora,
   niveles (Info/OK/Aviso/Error), categorías, filtro "Fallos y avisos",
   contador de eventos y limpieza.
 + Exportación del informe de fallos y errores en un archivo de texto
   con resumen, entorno (SO, escritorio, sesión, usuario, rutas),
   fallos/avisos y registro completo, en el idioma activo.
 + El registro se guarda en disco, se recarga al abrir la app y rota
   automáticamente a los 512 KB.
 + Corregido: arrastrar un .AppImage a la pestaña Integrar no lo
   agregaba; ahora cada archivo soltado se integra y queda anotado en
   el registro (los que no son .AppImage se avisan en lugar de
   ignorarse en silencio).
 + Corregido: al integrar por el botón del explorador la app no se
   actualizaba; ahora la biblioteca se refresca sola al terminar cada
   integración, análisis o actualización y la insignia "Integrada"
   aparece al momento.
 + Corregido: las opciones para crear el acceso directo en el menú de
   aplicaciones no reflejaban su resultado; ahora el estado se ve en la
   biblioteca, se refresca la caché del escritorio (update-desktop-
   database si existe) y volver a integrar la misma app ya no crea
   accesos duplicados ("naranjahub-foo (2).desktop").
 + Corregido: el botón Integrar quedaba escondido al abrir el
   explorador de archivos; ahora está siempre visible encima del
   explorador y toda la pestaña Integrar y Ajustes tienen scroll.
 + Registro de actividad y todos sus mensajes también en inglés.
 + Limpieza completa del código: eliminado el parser de zona horaria
   muerto y roto, reescrito con soporte correcto de horario de verano
   (transiciones e índices TZif v1/v2); eliminado estado innecesario de
   arrastre; cero avisos del compilador y de clippy.
 + --selftest ampliado: ahora son 16 simulaciones (registro de
   actividad, informe de fallos, accesos sin duplicados, zona horaria).

v1.0.0 — 21 de noviembre de 2026 — primera versión pública
 + Biblioteca de AppImages con búsqueda y orden por nombre/fecha/tamaño.
 + Integración con arrastrar y soltar y navegador de archivos interno.
 + Iconos y metadatos extraídos de la AppImage (sin FUSE).
 + Accesos directos .desktop para GNOME, KDE Plasma, COSMIC y XFCE.
 + Actualizaciones vía GitHub Releases (gh-releases-zsync) con descarga
   con progreso y reemplazo seguro.
 + Papelera freedesktop con .trashinfo y confirmación configurable.
 + Interfaz naranja/negro con logo propio y ventana con icono.
 + Idiomas español (por defecto) e inglés.
 + Pestaña Acerca de con autoría de Leyter Vázquez. V y licencia
   GNU GPL v3.
 + Modo --selftest con 13 simulaciones internas.
 + Entregables: AppImage, .deb, .run portable y código fuente.

----------------------------------------------------------------------
9. HOJA DE RUTA DE PRÓXIMAS VERSIONES
----------------------------------------------------------------------

Planificadas para futuras versiones (se documentarán en la sección 8
cuando se publiquen):
 - Temas claro/oscuro adicionales y acento configurable.
 - Más idiomas.
 - Estadísticas de uso y favoritos.
 - Verificación de firma/suma de control de las descargas.
 - Autostart de aplicaciones al iniciar sesión.
 - Vista de detalles ampliada por aplicación.
 - Niveles de detalle configurables en el Registro de actividad.

----------------------------------------------------------------------
10. LICENCIA GNU GPL v3
----------------------------------------------------------------------

NaranjaHub es software libre: puedes redistribuirlo y modificarlo bajo
los términos de la Licencia Pública General de GNU (GNU GPL) publicada
por la Free Software Foundation, en su versión 3 de la licencia.

Este programa se distribuye con la esperanza de que sea útil, pero SIN
NINGUNA GARANTÍA; sin siquiera la garantía implícita de COMERCIABILIDAD
o APTITUD PARA UN PROPÓSITO DETERMINADO.

El texto completo de la GNU GPL v3 se incluye en el archivo LICENSE que
acompaña a la aplicación y está disponible en:
https://www.gnu.org/licenses/gpl-3.0.html

Copyright (C) 2026 Leyter Vázquez. V

----------------------------------------------------------------------
11. ENGLISH SUMMARY
----------------------------------------------------------------------

NaranjaHub v1.4.1 is a free-software (GNU GPL v3) AppImage manager for
Linux, written in Rust with an orange/black egui interface. It works on
GNOME, KDE Plasma, COSMIC and XFCE (X11 and Wayland) with no external
dependencies: no FUSE, no GTK/Qt, no runtimes.

Features: AppImage library with search and sorting; drag-and-drop,
pasted-path and built-in file-browser integration into ~/Applications
with automatic icons and freedesktop menu entries; launching with one
click; GitHub Releases update checking (gh-releases-zsync read from
the .upd_info ELF section) with numeric version comparison, progress
download and backup-safe replacement; freedesktop trash support;
graphics backend selector (Auto/X11/Wayland); settings (language
English or Spanish, Spanish by default; apps folder; startup update
check; delete confirmation); About tab with "Created by Leyter
Vázquez. V" and the full GNU GPL v3 notice; CLI flags --version,
--help, --selftest (28 built-in simulations) and --inspect.

New in 1.4.0: in the Integrate tab the Browse button no longer lands on
top of the "Move instead of copy" option in narrow windows — options
and button now have their own rows — and the marked AppImage can be
unmarked at any time ("Unmark" button next to Integrate, visible even
with the browser closed, or clicking the file again in the browser).
In the Updates tab every app now has a "Set update address" option:
paste the GitHub repository URL (https://github.com/user/repo), just
user/repo, or the full gh-releases-zsync|user|repo|tag|pattern string;
if you omit the pattern it is derived from the local file name (the
version block becomes a wildcard, x86_64 is not mistaken for a
version). The address is stored on disk (~/.local/share/NaranjaHub/
updates.json, atomic writes), takes priority over the embedded update
information, can be edited or removed, follows the app when an update
renames the file, is cleared when the app goes to the trash, and
saving a valid address starts the check immediately.

New in 1.4.1: fixed the worst Updates-tab bug — checking for updates
WITHOUT an internet connection left the card stuck on "Checking..."
forever, because DNS/connection can hang far beyond the network agent
timeouts. There are now three layers of protection: the check agent
expires the whole request in 30 s (8 s to connect), a job watchdog in
the UI aborts any check still unfinished after 45 s ("timed out
checking (is there an internet connection?)"), and every worker job
runs inside catch_unwind so an unexpected panic shows an error instead
of freezing the card. Network failures are now reported in plain
language ("no internet connection (...)"); downloads show their own
"Downloading..." status; copies/downloads that make no progress for
120 s stop with a clear error; and Analyze no longer creates hollow
menu entries for AppImages deleted outside the app.

New in 1.3.0: drag-and-drop now works on Wayland sessions too — winit
only implements file-drop receiving on X11, so NaranjaHub starts on
X11/XWayland by default (Automatic mode) with automatic fallback to
native Wayland when XWayland is missing, plus a Settings selector and
the NARANJAHUB_BACKEND environment variable; update information is now
read from the .upd_info ELF section of the AppImage (the canonical
place appimagetool writes it, verified against a real appimagetool-
built AppImage), so GitHub-hosted apps are found; versions are compared
numerically so the same version is no longer offered as "new" just
because the local file name differs; update installs are backup-safe
(in-place replacement when the asset name matches, full rollback on
failure); "Check all" auto-analyzes apps without update info and chains
into a check; new paste-a-path integration input (plain paths, ~ and
file:// URIs); new --inspect CLI command; "Move instead of copy" now
renames atomically; dead v1.0-era files removed and the .deb version is
taken from Cargo.toml. 42 unit tests and 25 built-in simulations pass;
zero compiler/clippy/rustdoc warnings.

New in 1.1.0: an Activity log tab recording every action in detail
(timestamp, level, category) with a persistent on-disk log, a filter
for errors and warnings, and one-click export of a full error report
(summary, environment, recorded failures and complete log) as a text
file. Also fixed: drag-and-drop integration, library refresh after
integrating, menu-shortcut creation feedback (no more duplicate .desktop
files), and the hidden Integrate button (the tab now scrolls and the
button always stays visible).

Deliverables: NaranjaHub-1.4.1-x86_64.AppImage (portable, FUSE-free via
--appimage-extract-and-run), naranjahub_1.4.1_amd64.deb (no package
dependencies), NaranjaHub-1.4.1-x86_64.run (self-extracting user-space
installer: --portable, --dir, --no-menu, --uninstall), and the full
source code in a zip. This readme is included in every package and its
changelog (section 8) will be updated with every release.

Report ideas or problems to the author: Leyter Vázquez. V.
