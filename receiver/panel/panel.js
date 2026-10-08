// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>
'use strict';

/*
 * x-link-collector panel. Plain script, no build step: the receiver serves this
 * file as is from inside its own binary, and it talks to nothing but the
 * receiver's /api/ on the same origin.
 */

/* ----------------------------------------------------------------- words */

const WORDS = {
  en: {
    'skip': 'Skip to content',
    'nav.overview': 'Overview',
    'nav.links': 'Links',
    'nav.download': 'Download',
    'nav.folder': 'Folder',
    'nav.settings': 'Settings',
    'nav.label': 'Sections',
    'conn.ok': 'Running on port {port}',
    'conn.lost': 'Not reachable',
    'offline': 'The panel cannot reach x-link-receiver. It may have stopped: start it again and this page carries on by itself.',
    'restart': 'Settings say port {want}, but x-link-receiver is still on {port}. Restart it to switch.',
    'theme.toDark': 'Switch to the dark theme',
    'theme.toLight': 'Switch to the light theme',
    'lang.name': 'Türkçe',
    'lang.label': 'Türkçe arayüze geç',

    'ov.title': ['{n} link collected', '{n} links collected'],
    'ov.titleEmpty': 'Nothing collected yet',
    'ov.subEmpty': 'Middle-click a tweet in your browser. The link lands here and the tab closes by itself.',
    'ov.subWaiting': ['{n} is waiting to be downloaded.', '{n} are waiting to be downloaded.'],
    'ov.subRetrying': '{n} waiting to be downloaded, plus {k} that failed before and will be tried again.',
    'ov.subRetryOnly': ['{n} link that failed before will be tried again.', '{n} links that failed before will be tried again.'],
    'ov.subFailed': ['Everything else is downloaded; {n} link failed.', 'Everything else is downloaded; {n} links failed.'],
    'ov.subUnfinished': 'Everything else is downloaded; {failed} failed and {nomedia} had no media.',
    'ov.subDone': 'Everything is downloaded. New links show up here as you collect them.',
    'ov.subRunning': 'A download is running; squares fill in as links finish.',
    'ov.subAuto': 'New links download by themselves shortly after you collect them.',
    'go.download': ['Download {n} link', 'Download {n} links'],
    'go.retry': ['Try {n} link again', 'Try {n} links again'],
    'go.nothing': 'Nothing to download',
    'go.running': 'Downloading {done} of {total}',
    'go.starting': 'Starting the download',
    'go.busy': 'Another job is running',
    'state.done': 'Downloaded',
    'state.no-media': 'No media',
    'state.failed': 'Failed',
    'state.pending': 'Waiting',
    'strip.note': 'Oldest link first, one square each',
    'strip.noteGrouped': 'Oldest link first, one square for every {k} links',
    'strip.label': '{total} links: {done} downloaded, {nomedia} without media, {failed} failed, {waiting} waiting.',
    'strip.line': 'Line {n} of {total}',
    'strip.lines': 'Lines {from} to {to} of {total}',
    'ov.perDay': 'Collected per day',
    'ov.daysAgo': '14 days ago',
    'ov.today': 'Today',
    'ov.dayCount': ['{n} link', '{n} links'],
    'ov.quiet': 'Nothing collected in the last 14 days.',
    'ov.latest': 'Latest',
    'ov.allLinks': 'All links',
    'ov.latestEmpty': 'Links you collect appear here.',

    'setup.title': 'Finish setting up',
    'setup.ext': 'Load the browser extension',
    'setup.extHow': 'Open chrome://extensions (or vivaldi://, brave://, edge://extensions), turn on Developer mode, choose Load unpacked and pick this folder:',
    'setup.tools': 'Install the download tools',
    'setup.toolsHow': 'yt-dlp and gallery-dl do the actual downloading. They go into downloader/.venv; nothing is installed system-wide.',
    'setup.python': 'Install Python 3',
    'setup.pythonHow': 'The downloader is a Python script. Get Python from python.org (on Windows, tick "Add python.exe to PATH"), then press Check again under Settings.',
    'setup.install': 'Install the tools',
    'copy': 'Copy',
    'copied': 'Copied',

    'links.title': 'Links',
    'links.search': 'Search by account, tweet id or address',
    'links.filterLabel': 'Show',
    'links.all': 'All',
    'links.add': 'Add links',
    'links.addHelp': 'Paste links, one per line. Tweet links are cleaned the same way the extension cleans them, and ones already collected are skipped.',
    'links.addSave': 'Add to links.txt',
    'links.addCancel': 'Close',
    'links.addedNew': ['{n} link added', '{n} links added'],
    'links.addedKnown': ['{n} was already there', '{n} were already there'],
    'links.rejected': ['{n} line was not a link', '{n} lines were not links'],
    'links.empty': 'No links yet. Collect some with the extension, or paste them with Add links.',
    'links.noMatch': 'No links match.',
    'links.line': 'line {n}',
    'links.posted': 'posted {date}',
    'links.collected': 'collected {ago}',
    'links.unknownAuthor': 'account unknown',
    'links.open': 'Open on X',
    'links.openOther': 'Open',
    'links.download': 'Download',
    'links.retry': 'Try again',
    'links.show': 'Show the media',
    'viewer.prev': 'Previous',
    'viewer.next': 'Next',
    'viewer.close': 'Close',
    'viewer.count': '{i} of {n}',
    'notify.done': 'Download finished',
    'notify.stopped': 'Download stopped',
    'notify.got': ['{n} link downloaded', '{n} links downloaded'],
    'notify.failed': ['{n} could not be downloaded', '{n} could not be downloaded'],
    'notify.nothing': 'Nothing new was downloaded.',
    'setup.update': 'Update the download tools',
    'setup.titleUpkeep': 'One thing to see to',
    'setup.updateOld': 'yt-dlp is {days} days old. X changes often, and older versions stop working after a while.',
    'setup.updateFailing': 'The last download fetched nothing and several links failed. A newer yt-dlp usually fixes that.',
    'setup.updateNow': 'Update now',
    'st.released': 'released {ago}',
    'links.copy': 'Copy',
    'links.remove': 'Remove',
    'links.removeTitle': 'Remove this link?',
    'links.removeBody': 'It comes out of links.txt. Anything already downloaded for it stays where it is.',
    'links.removed': 'Removed from links.txt',
    'links.more': 'Show more',
    'links.shown': '{shown} of {matched}',

    'dl.title': 'Download media',
    'dl.lead': 'Fetches the photos and videos behind every link that has not been downloaded yet, at the best quality X serves. Stopping is safe: the next run carries on where this one left off.',
    'dl.route': 'Where to fetch from',
    'route.off': 'X only',
    'route.fallback': 'X, then mirrors',
    'route.only': 'Mirrors only',
    'route.offHelp': 'Straight from X. Age-restricted tweets need a login for this.',
    'route.fallbackHelp': 'When X refuses a tweet, fxtwitter and vxtwitter are asked for it. They see the tweet id and nothing else.',
    'route.onlyHelp': 'Skips X and asks the mirrors right away. Much faster when every link is age-restricted.',
    'dl.jobs': 'Parallel downloads',
    'dl.jobsHelp': 'X starts refusing above 5.',
    'dl.cookies': 'Log in with',
    'cookies.none': 'No login',
    'cookies.file': 'A cookies.txt file',
    'dl.cookiesHelp': 'Your own X session also reaches protected accounts you follow. It is read from the browser and sent only to X.',
    'dl.cookiesPath': 'Path to cookies.txt',
    'dl.once': 'For this run only',
    'dl.retry': 'Also try links that failed or had no media',
    'dl.limit': 'Stop after this many links',
    'dl.limitHelp': 'Leave empty to do them all. The rest stay queued for the next run.',
    'dl.preview': 'Preview',
    'dl.auto': 'Download new links automatically',
    'dl.autoHelp': 'Starts 20 seconds after the last link you collect, so a burst of tabs becomes one run.',
    'dl.saved': 'Saved',

    'job.title.download': 'Download',
    'job.title.flatten': 'Flatten',
    'job.title.unflatten': 'Undo flatten',
    'job.title.setup': 'Installing the download tools',
    'job.state.running': 'Running',
    'job.state.stopping': 'Stopping',
    'job.state.done': 'Finished',
    'job.state.stopped': 'Stopped',
    'job.state.failed': 'Ended with an error',
    'job.progress': '{done} of {total}',
    'job.left': 'about {t} left',
    'job.exited': 'exited with code {code}',
    'job.stop': 'Stop',
    'job.idle': 'Nothing is running. The output of the next job appears here.',
    'job.output': 'Output',
    'job.startedDownload': 'Download started',
    'job.startedPreview': 'Preview started',

    'fo.title': 'Media folder',
    'fo.open': 'Open folder',
    'fo.files': ['{n} file, {size}', '{n} files, {size}'],
    'fo.kinds': '{videos} videos and {images} images',
    'fo.none': 'Nothing downloaded yet.',
    'fo.flat': 'Put everything in one folder',
    'fo.flatHelp': 'Moves every file out of its per-account folder into one place. Every move is recorded, so Undo puts each file back exactly.',
    'fo.dest': 'Destination',
    'fo.prefix': 'Keep the account name in the file name',
    'fo.videos': 'Videos only',
    'fo.copy': 'Copy instead of moving (needs twice the space)',
    'fo.preview': 'Preview',
    'fo.flatten': 'Flatten',
    'fo.undo': 'Undo flatten',
    'fo.flattenTitle': 'Flatten the media folder?',
    'fo.flattenBody': 'Files move into {dest}. You can undo this afterwards.',
    'fo.undoTitle': 'Put the files back?',
    'fo.undoBody': 'Every file returns to the account folder it came from.',

    'st.title': 'Settings',
    'st.files': 'Files',
    'st.links_file': 'Links file',
    'st.links_fileHelp': 'Where collected links are written, one per line.',
    'st.media_dir': 'Media folder',
    'st.media_dirHelp': 'Where downloads are saved, one folder per account.',
    'st.downloading': 'Downloading',
    'st.mirror': 'Where to fetch from',
    'st.jobs': 'Parallel downloads',
    'st.cookies': 'Log in with',
    'st.timeout': 'Give up on a link after',
    'st.timeoutHelp': 'A link still going after this long is stopped and tried again next run.',
    'st.seconds': 'seconds',
    'st.metadata': "Save each tweet's details next to its media",
    'st.metadataHelp': 'A JSON file per tweet: text, author, date.',
    'st.auto_download': 'Download new links automatically',
    'st.flatten': 'Flattening',
    'st.flatten_dest': 'Destination',
    'st.flatten_prefix_handle': 'Keep the account name in the file name',
    'st.flatten_videos_only': 'Videos only',
    'st.flatten_copy': 'Copy instead of moving',
    'st.collector': 'Receiver',
    'st.port': 'Port',
    'st.portHelp': 'Takes effect when x-link-receiver restarts. Set the same port in the extension too: right-click its icon, then Options.',
    'st.fsync': 'Make sure each link is on disk before its tab closes',
    'st.fsyncHelp': 'The safest choice. Turn it off only on a very slow disk.',
    'st.where': 'Where the tools are',
    'st.tools_dir': 'Tools folder',
    'st.tools_dirHelp': 'The folder with downloader/ and spliter/ in it.',
    'st.python': 'Python',
    'st.pythonHelp': 'Leave empty to use the one on this computer.',
    'st.tools': 'Download tools',
    'st.check': 'Check again',
    'st.install': 'Install or update',
    'st.checking': 'Checking…',
    'st.found': 'Found in {where}',
    'st.venv': 'downloader/.venv',
    'st.path': 'the system PATH',
    'st.missing': 'Not installed',
    'st.ffmpegMissing': 'Not installed; needed when X serves sound and picture separately',
    'st.notFound': 'Not found',
    'st.about': 'About',
    'st.version': 'Version',
    'st.configFile': 'Settings file',
    'st.linksFileNow': 'Links file in use',
    'st.source': 'Source and help',
    'st.openFolder': 'Open folder',
    'st.pinned': 'Fixed when x-link-receiver was started (command line or environment); change it there.',
    'st.autostart': 'Start when you sign in to Windows',
    'st.autostartHelp': 'It runs quietly in the background; the panel is always at this address.',
    'st.quit': 'Quit x-link-receiver',
    'st.quitHelp': 'No links are collected until it is started again. A running download is stopped first.',
    'st.quitTitle': 'Quit x-link-receiver?',
    'st.quitDone': 'x-link-receiver has quit.',
    'err.autostart': 'Starting at sign-in could not be changed ({reason}).',
    'save.unsaved': 'Unsaved changes',
    'save.discard': 'Discard',
    'save.save': 'Save changes',
    'save.saved': 'Settings saved',
    'cancel': 'Cancel',

    'err.network': 'x-link-receiver did not answer.',
    'err.busy': 'Another job is running. Let it finish, or stop it first.',
    'err.no_tools': 'The downloader/ and spliter/ folders were not found. Choose the tools folder in Settings.',
    'err.script_missing': 'This file is missing: {path}. Check the tools folder in Settings.',
    'err.no_python': 'Python 3 was not found. Install it, or give its path in Settings.',
    'err.python_missing': 'There is no Python here: {path}. Check it in Settings.',
    'err.links_file': 'This links file cannot be used: {path} ({reason})',
    'err.save': 'The settings could not be saved: {path} ({reason})',
    'err.open': 'Could not open {path} ({reason})',
    'err.rewrite': 'links.txt could not be rewritten ({reason}).',
    'err.write': 'The link could not be written to the file ({reason}).',
    'err.no_url': 'None of that was a link.',
    'err.empty': 'Paste one or more links first.',
    'reason.denied': 'permission denied',
    'reason.missing': 'no such place',
    'reason.no_opener': 'no program to open it with',
  },

  tr: {
    'skip': 'İçeriğe geç',
    'nav.overview': 'Genel bakış',
    'nav.links': 'Bağlantılar',
    'nav.download': 'İndirme',
    'nav.folder': 'Klasör',
    'nav.settings': 'Ayarlar',
    'nav.label': 'Bölümler',
    'conn.ok': '{port} portunda çalışıyor',
    'conn.lost': 'Ulaşılamıyor',
    'offline': 'Panel x-link-receiver programına ulaşamıyor. Program durmuş olabilir; yeniden başlattığınızda bu sayfa kendiliğinden devam eder.',
    'restart': 'Ayarlarda {want} portu seçili, ama x-link-receiver hâlâ {port} portunda çalışıyor. Yeni porta geçmesi için programı yeniden başlatın.',
    'theme.toDark': 'Koyu temaya geç',
    'theme.toLight': 'Açık temaya geç',
    'lang.name': 'English',
    'lang.label': 'Switch to English',

    'ov.title': '{n} bağlantı toplandı',
    'ov.titleEmpty': 'Henüz bağlantı toplanmadı',
    'ov.subEmpty': 'Tarayıcıda bir tweet’e orta tıklayın: bağlantı buraya kaydedilir, sekme de kendiliğinden kapanır.',
    'ov.subWaiting': '{n} bağlantı indirilmeyi bekliyor.',
    'ov.subRetrying': '{n} bağlantı indirilmeyi bekliyor; daha önce indirilemeyen {k} bağlantı da yeniden denenecek.',
    'ov.subRetryOnly': 'Daha önce indirilemeyen {n} bağlantı yeniden denenecek.',
    'ov.subFailed': 'Geri kalanların hepsi indirildi; {n} bağlantı indirilemedi.',
    'ov.subUnfinished': 'Geri kalanların hepsi indirildi; {failed} bağlantı indirilemedi, {nomedia} bağlantıda medya yoktu.',
    'ov.subDone': 'Hepsi indirildi. Yeni topladığınız bağlantılar burada görünecek.',
    'ov.subRunning': 'İndirme sürüyor; her bağlantı indirildikçe karesi renkleniyor.',
    'ov.subAuto': 'Yeni bağlantılar, toplandıktan kısa bir süre sonra kendiliğinden indirilir.',
    'go.download': '{n} bağlantıyı indir',
    'go.retry': '{n} bağlantıyı yeniden dene',
    'go.nothing': 'İndirilecek bağlantı yok',
    'go.running': 'İndiriliyor: {done} / {total}',
    'go.starting': 'İndirme başlıyor',
    'go.busy': 'Başka bir iş çalışıyor',
    'state.done': 'İndirildi',
    'state.no-media': 'Medya yok',
    'state.failed': 'İndirilemedi',
    'state.pending': 'Bekliyor',
    'strip.note': 'En eski bağlantı başta; her kare bir bağlantı',
    'strip.noteGrouped': 'En eski bağlantı başta; her kare {k} bağlantı',
    'strip.label': '{total} bağlantı: {done} indirildi, {nomedia} bağlantıda medya yok, {failed} indirilemedi, {waiting} bekliyor.',
    'strip.line': 'Satır {n} / {total}',
    'strip.lines': 'Satır {from}–{to} / {total}',
    'ov.perDay': 'Günlük toplanan bağlantılar',
    'ov.daysAgo': '14 gün önce',
    'ov.today': 'Bugün',
    'ov.dayCount': '{n} bağlantı',
    'ov.quiet': 'Son 14 günde hiç bağlantı toplanmadı.',
    'ov.latest': 'Son eklenenler',
    'ov.allLinks': 'Tüm bağlantılar',
    'ov.latestEmpty': 'Topladığınız bağlantılar burada görünür.',

    'setup.title': 'Kurulumu tamamlayın',
    'setup.ext': 'Tarayıcı eklentisini yükleyin',
    'setup.extHow': 'Tarayıcınızın eklentiler sayfasını açın (chrome://extensions, vivaldi://extensions, brave://extensions ya da edge://extensions). Geliştirici modunu açın, Paketlenmemiş öğe yükle düğmesine basın ve şu klasörü seçin:',
    'setup.tools': 'İndirme araçlarını kurun',
    'setup.toolsHow': 'Asıl indirmeyi yt-dlp ve gallery-dl yapar. İkisi de downloader/.venv klasörüne kurulur; sisteminize başka bir şey kurulmaz.',
    'setup.python': 'Python 3 kurun',
    'setup.pythonHow': 'İndirici bir Python programıdır. python.org adresinden Python indirip kurun (Windows’ta kurulum sırasında “Add python.exe to PATH” kutusunu işaretleyin), ardından Ayarlar sayfasındaki Yeniden kontrol et düğmesine basın.',
    'setup.install': 'Araçları kur',
    'copy': 'Kopyala',
    'copied': 'Kopyalandı',

    'links.title': 'Bağlantılar',
    'links.search': 'Hesap adı, tweet kimliği ya da adres arayın',
    'links.filterLabel': 'Göster',
    'links.all': 'Tümü',
    'links.add': 'Bağlantı ekle',
    'links.addHelp': 'Bağlantıları her satıra bir tane gelecek şekilde yapıştırın. Tweet bağlantıları eklentide olduğu gibi temizlenir; daha önce toplanmış olanlar atlanır.',
    'links.addSave': 'links.txt dosyasına ekle',
    'links.addCancel': 'Kapat',
    'links.addedNew': '{n} bağlantı eklendi',
    'links.addedKnown': '{n} bağlantı zaten vardı',
    'links.rejected': '{n} satır bağlantı değildi',
    'links.empty': 'Henüz bağlantı yok. Eklentiyle toplayın ya da Bağlantı ekle düğmesiyle yapıştırın.',
    'links.noMatch': 'Eşleşen bağlantı yok.',
    'links.line': 'satır {n}',
    'links.posted': 'Paylaşıldı: {date}',
    'links.collected': 'Toplandı: {ago}',
    'links.unknownAuthor': 'hesap bilinmiyor',
    'links.open': 'X’te aç',
    'links.openOther': 'Aç',
    'links.download': 'İndir',
    'links.retry': 'Yeniden dene',
    'links.show': 'Medyayı göster',
    'viewer.prev': 'Önceki',
    'viewer.next': 'Sonraki',
    'viewer.close': 'Kapat',
    'viewer.count': '{i} / {n}',
    'notify.done': 'İndirme bitti',
    'notify.stopped': 'İndirme durduruldu',
    'notify.got': '{n} bağlantı indirildi',
    'notify.failed': '{n} bağlantı indirilemedi',
    'notify.nothing': 'Yeni bir şey indirilmedi.',
    'setup.update': 'İndirme araçlarını güncelleyin',
    'setup.titleUpkeep': 'Yapmanız gereken bir şey var',
    'setup.updateOld': 'Kullandığınız yt-dlp {days} gün önce yayımlandı. X sık sık değiştiği için eski sürümler bir süre sonra çalışmaz hale gelir.',
    'setup.updateFailing': 'Son indirmede hiçbir dosya inmedi ve birkaç bağlantı indirilemedi. Bu sorun çoğu zaman yt-dlp güncellenince düzelir.',
    'setup.updateNow': 'Şimdi güncelle',
    'st.released': '{ago} yayımlandı',
    'links.copy': 'Kopyala',
    'links.remove': 'Kaldır',
    'links.removeTitle': 'Bu bağlantı kaldırılsın mı?',
    'links.removeBody': 'Bağlantı links.txt dosyasından çıkarılır. Daha önce indirilmiş dosyaları silinmez.',
    'links.removed': 'Bağlantı links.txt dosyasından kaldırıldı',
    'links.more': 'Daha fazla göster',
    'links.shown': '{matched} bağlantıdan {shown} tanesi gösteriliyor',

    'dl.title': 'Medyayı indir',
    'dl.lead': 'Henüz indirilmemiş bağlantılardaki fotoğraf ve videoları, X’in sunduğu en yüksek kalitede indirir. İndirmeyi istediğiniz zaman durdurabilirsiniz; bir sonraki indirme kaldığı yerden devam eder.',
    'dl.route': 'Nereden indirilsin',
    'route.off': 'Yalnızca X',
    'route.fallback': 'Önce X, sonra aynalar',
    'route.only': 'Yalnızca aynalar',
    'route.offHelp': 'Doğrudan X’ten indirir. Yaş sınırlı tweet’ler bu yolla yalnızca oturum açıkken indirilebilir.',
    'route.fallbackHelp': 'X bir tweet’i vermezse ayna sunuculara (fxtwitter ve vxtwitter) sorulur. Bu servisler yalnızca tweet kimliğini görür.',
    'route.onlyHelp': 'X’i atlayıp doğrudan ayna sunuculara sorar. Bağlantıların hepsi yaş sınırlıysa çok daha hızlıdır.',
    'dl.jobs': 'Eşzamanlı indirme sayısı',
    'dl.jobsHelp': '5’ten fazlasında X istekleri reddetmeye başlar.',
    'dl.cookies': 'Kullanılacak oturum',
    'cookies.none': 'Oturum kullanma',
    'cookies.file': 'cookies.txt dosyası',
    'dl.cookiesHelp': 'Kendi X oturumunuzla, takip ettiğiniz korumalı hesaplara da erişilebilir. Oturum bilgisi tarayıcıdan okunur ve yalnızca X’e gönderilir.',
    'dl.cookiesPath': 'cookies.txt dosyasının yolu',
    'dl.once': 'Yalnızca bu indirme için',
    'dl.retry': 'İndirilemeyenleri ve medyası olmayanları da yeniden dene',
    'dl.limit': 'Şu kadar bağlantıdan sonra dur',
    'dl.limitHelp': 'Hepsini indirmek için boş bırakın. Kalanlar bir sonraki indirmeyi bekler.',
    'dl.preview': 'Önizle',
    'dl.auto': 'Yeni bağlantıları kendiliğinden indir',
    'dl.autoHelp': 'Son bağlantıyı topladıktan 20 saniye sonra başlar; art arda açtığınız sekmeler tek seferde indirilir.',
    'dl.saved': 'Kaydedildi',

    'job.title.download': 'İndirme',
    'job.title.flatten': 'Tek klasöre taşıma',
    'job.title.unflatten': 'Taşımayı geri alma',
    'job.title.setup': 'İndirme araçlarının kurulumu',
    'job.state.running': 'Çalışıyor',
    'job.state.stopping': 'Durduruluyor',
    'job.state.done': 'Bitti',
    'job.state.stopped': 'Durduruldu',
    'job.state.failed': 'Hatayla bitti',
    'job.progress': '{done} / {total}',
    'job.left': 'yaklaşık {t} kaldı',
    'job.exited': '{code} koduyla sonlandı',
    'job.stop': 'Durdur',
    'job.idle': 'Şu anda çalışan bir iş yok. Bir sonraki işin çıktısı burada görünür.',
    'job.output': 'Çıktı',
    'job.startedDownload': 'İndirme başladı',
    'job.startedPreview': 'Önizleme başladı',

    'fo.title': 'Medya klasörü',
    'fo.open': 'Klasörü aç',
    'fo.files': '{n} dosya, {size}',
    'fo.kinds': '{videos} video, {images} görsel',
    'fo.none': 'Henüz bir şey indirilmedi.',
    'fo.flat': 'Hepsini tek klasörde topla',
    'fo.flatHelp': 'Her dosyayı kendi hesap klasöründen çıkarıp tek bir klasöre taşır. Her taşıma kaydedilir; Taşımayı geri al düğmesi her dosyayı eski yerine koyar.',
    'fo.dest': 'Hedef klasör',
    'fo.prefix': 'Dosya adında hesap adı kalsın',
    'fo.videos': 'Yalnızca videolar',
    'fo.copy': 'Taşımak yerine kopyala (iki kat yer kaplar)',
    'fo.preview': 'Önizle',
    'fo.flatten': 'Tek klasöre taşı',
    'fo.undo': 'Taşımayı geri al',
    'fo.flattenTitle': 'Dosyalar tek klasörde toplansın mı?',
    'fo.flattenBody': 'Dosyalar şu klasöre taşınır: {dest}. Bu işlemi sonradan geri alabilirsiniz.',
    'fo.undoTitle': 'Dosyalar eski yerlerine taşınsın mı?',
    'fo.undoBody': 'Her dosya geldiği hesap klasörüne geri döner.',

    'st.title': 'Ayarlar',
    'st.files': 'Dosyalar',
    'st.links_file': 'Bağlantı dosyası',
    'st.links_fileHelp': 'Toplanan bağlantıların her satıra bir tane olacak şekilde yazıldığı dosya.',
    'st.media_dir': 'Medya klasörü',
    'st.media_dirHelp': 'İndirilenlerin kaydedildiği yer; her hesap için ayrı bir klasör açılır.',
    'st.downloading': 'İndirme',
    'st.mirror': 'Nereden indirilsin',
    'st.jobs': 'Eşzamanlı indirme sayısı',
    'st.cookies': 'Kullanılacak oturum',
    'st.timeout': 'Bir bağlantı için zaman aşımı',
    'st.timeoutHelp': 'Bu süreyi aşan bağlantı durdurulur ve bir sonraki indirmede yeniden denenir.',
    'st.seconds': 'saniye',
    'st.metadata': 'Her tweet’in bilgilerini medyasının yanına kaydet',
    'st.metadataHelp': 'Her tweet için bir JSON dosyası: metin, hesap ve tarih.',
    'st.auto_download': 'Yeni bağlantıları kendiliğinden indir',
    'st.flatten': 'Tek klasöre toplama',
    'st.flatten_dest': 'Hedef klasör',
    'st.flatten_prefix_handle': 'Dosya adında hesap adı kalsın',
    'st.flatten_videos_only': 'Yalnızca videolar',
    'st.flatten_copy': 'Taşımak yerine kopyala',
    'st.collector': 'Alıcı',
    'st.port': 'Port',
    'st.portHelp': 'x-link-receiver yeniden başlatıldığında geçerli olur. Eklentide de aynı portu ayarlayın: eklenti simgesine sağ tıklayıp Seçenekler’i seçin.',
    'st.fsync': 'Sekme kapanmadan önce her bağlantının diske yazıldığından emin ol',
    'st.fsyncHelp': 'En güvenli seçenek budur. Yalnızca çok yavaş bir diskte kapatın.',
    'st.where': 'Araçların yeri',
    'st.tools_dir': 'Araç klasörü',
    'st.tools_dirHelp': 'İçinde downloader/ ve spliter/ klasörleri bulunan klasör.',
    'st.python': 'Python',
    'st.pythonHelp': 'Bilgisayarınızda kurulu olanı kullanmak için boş bırakın.',
    'st.tools': 'İndirme araçları',
    'st.check': 'Yeniden kontrol et',
    'st.install': 'Kur ya da güncelle',
    'st.checking': 'Kontrol ediliyor…',
    'st.found': 'Bulunduğu yer: {where}',
    'st.venv': 'downloader/.venv',
    'st.path': 'sistemdeki PATH',
    'st.missing': 'Kurulu değil',
    'st.ffmpegMissing': 'Kurulu değil; X sesi ve görüntüyü ayrı verdiğinde gerekir',
    'st.notFound': 'Bulunamadı',
    'st.about': 'Hakkında',
    'st.version': 'Sürüm',
    'st.configFile': 'Ayar dosyası',
    'st.linksFileNow': 'Kullanılan bağlantı dosyası',
    'st.source': 'Kaynak kod ve yardım',
    'st.openFolder': 'Klasörü aç',
    'st.pinned': 'Bu ayar, x-link-receiver başlatılırken komut satırında ya da bir ortam değişkeniyle belirlendi; oradan değiştirilebilir.',
    'st.autostart': 'Windows açılınca kendiliğinden başlat',
    'st.autostartHelp': 'Arka planda sessizce çalışır; panel her zaman bu adreste olur.',
    'st.quit': 'x-link-receiver programını kapat',
    'st.quitHelp': 'Program yeniden başlatılana kadar bağlantı toplanmaz. Süren bir indirme varsa önce o durdurulur.',
    'st.quitTitle': 'x-link-receiver kapatılsın mı?',
    'st.quitDone': 'x-link-receiver kapatıldı.',
    'err.autostart': 'Açılışta başlatma ayarı değiştirilemedi ({reason}).',
    'save.unsaved': 'Kaydedilmemiş değişiklikler var',
    'save.discard': 'Vazgeç',
    'save.save': 'Değişiklikleri kaydet',
    'save.saved': 'Ayarlar kaydedildi',
    'cancel': 'Vazgeç',

    'err.network': 'x-link-receiver yanıt vermedi.',
    'err.busy': 'Başka bir iş çalışıyor. Bitmesini bekleyin ya da önce onu durdurun.',
    'err.no_tools': 'downloader/ ve spliter/ klasörleri bulunamadı. Araç klasörünü Ayarlar sayfasından seçin.',
    'err.script_missing': 'Şu dosya bulunamadı: {path}. Ayarlar sayfasındaki araç klasörünü kontrol edin.',
    'err.no_python': 'Python 3 bulunamadı. Kurun ya da yolunu Ayarlar sayfasında belirtin.',
    'err.python_missing': 'Şu konumda Python yok: {path}. Ayarlar sayfasından kontrol edin.',
    'err.links_file': 'Bu bağlantı dosyası kullanılamıyor: {path} ({reason})',
    'err.save': 'Ayarlar kaydedilemedi: {path} ({reason})',
    'err.open': 'Açılamadı: {path} ({reason})',
    'err.rewrite': 'links.txt dosyası yeniden yazılamadı ({reason}).',
    'err.write': 'Bağlantı dosyaya yazılamadı ({reason}).',
    'err.no_url': 'Yapıştırdıklarınızın hiçbiri bağlantı değildi.',
    'err.empty': 'Önce bir ya da daha fazla bağlantı yapıştırın.',
    'reason.denied': 'izin yok',
    'reason.missing': 'böyle bir yer yok',
    'reason.no_opener': 'açacak bir program bulunamadı',
  },
};

/* -------------------------------------------------------------- plumbing */

const $ = (selector) => document.querySelector(selector);

/** Browser storage holds preferences only, and may be missing or blocked. */
function remember(key, value) {
  try {
    if (value === undefined) return localStorage.getItem(`x-link-collector.${key}`);
    localStorage.setItem(`x-link-collector.${key}`, value);
  } catch {
    // fine without it
  }
  return null;
}

let lang = remember('lang') || ((navigator.language || '').toLowerCase().startsWith('tr') ? 'tr' : 'en');
if (!WORDS[lang]) lang = 'en';
let fmt = null;

function makeFormatters() {
  const locale = lang === 'tr' ? 'tr-TR' : 'en-GB';
  fmt = {
    number: new Intl.NumberFormat(locale),
    decimal: new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }),
    date: new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }),
    day: new Intl.DateTimeFormat(locale, { weekday: 'short', day: 'numeric', month: 'short' }),
    relative: new Intl.RelativeTimeFormat(locale, { numeric: 'auto' }),
  };
}

/** A string from WORDS; `{name}` is filled from `vars`, `vars.n` picks a plural. */
function t(key, vars = {}) {
  let text = WORDS[lang][key] ?? WORDS.en[key] ?? key;
  if (Array.isArray(text)) text = vars.n === 1 ? text[0] : text[1];
  return text.replace(/\{(\w+)\}/g, (whole, name) => {
    if (!(name in vars)) return whole;
    const value = vars[name];
    return typeof value === 'number' ? fmt.number.format(value) : String(value);
  });
}

function h(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === null || value === false) continue;
    if (key === 'class') node.className = value;
    else if (key === 'text') node.textContent = value;
    else if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? '' : value);
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child instanceof Node ? child : String(child));
  }
  return node;
}

function icon(name) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('class', 'icon');
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(ns, 'use');
  use.setAttribute('href', `#i-${name}`);
  svg.append(use);
  return svg;
}

/**
 * A refusal from the receiver carries a code and the values its sentence
 * needs; `explain` puts it into the reader's language. The English `error`
 * text stands in for anything without a code.
 */
async function request(path, init = {}) {
  let res;
  try {
    res = await fetch(path, { cache: 'no-store', ...init });
  } catch {
    throw Object.assign(new Error(WORDS.en['err.network']), { code: 'network' });
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw Object.assign(new Error(data.error || `HTTP ${res.status}`), { code: data.code, vars: data.vars });
  }
  return data;
}

const getJSON = (path) => request(path);

const postJSON = (path, body) => request(path, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(body ?? {}),
});

function explain(error) {
  const key = `err.${error.code}`;
  if (!error.code || !(key in WORDS.en)) return error.message;
  const vars = { ...(error.vars || {}) };
  if ('reason' in vars) {
    const reason = `reason.${vars.reason}`;
    vars.reason = vars.reason && reason in WORDS.en ? t(reason) : vars.detail || '';
  }
  return t(key, vars);
}

function toast(message, kind = '') {
  const node = h('div', { class: `toast ${kind}`, role: kind === 'error' ? 'alert' : 'status', text: message });
  $('#toasts').append(node);
  setTimeout(() => node.remove(), kind === 'error' ? 8000 : 3500);
}

function confirmAction({ title, body, action }) {
  const dialog = $('#confirm');
  $('#confirm-title').textContent = title;
  $('#confirm-body').textContent = body;
  $('#confirm-ok').textContent = action;
  $('#confirm-cancel').textContent = t('cancel');
  return new Promise((resolve) => {
    const done = (answer) => {
      dialog.close();
      resolve(answer);
    };
    $('#confirm-ok').onclick = () => done(true);
    $('#confirm-cancel').onclick = () => done(false);
    dialog.oncancel = () => resolve(false);
    dialog.showModal();
  });
}

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    toast(t('copied'));
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* ------------------------------------------------------------ formatting */

const num = (n) => fmt.number.format(n);

/** The receiver's clock, so "2 minutes ago" is right even if this one is off. */
function serverNow() {
  const s = state.status;
  return s ? s.now + (Date.now() - state.fetchedAt) / 1000 : Date.now() / 1000;
}

function ago(unix) {
  const diff = unix - serverNow();
  const abs = Math.abs(diff);
  if (abs < 45) return fmt.relative.format(0, 'second');
  const steps = [[3600, 60, 'minute'], [86400, 3600, 'hour'], [2592000, 86400, 'day'], [31536000, 2592000, 'month']];
  for (const [limit, size, unit] of steps) {
    if (abs < limit) return fmt.relative.format(Math.round(diff / size), unit);
  }
  return fmt.relative.format(Math.round(diff / 31536000), 'year');
}

function duration(seconds) {
  const tr = lang === 'tr';
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))} ${tr ? 'sn' : 's'}`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} ${tr ? 'dk' : 'min'}`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return tr ? `${hours} sa ${minutes} dk` : `${hours} h ${minutes} min`;
}

function bytes(n) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i += 1;
  }
  return `${i ? fmt.decimal.format(n) : fmt.number.format(n)} ${units[i]}`;
}

/** Tweet ids since late 2010 carry their creation time in the top bits. */
function postedAt(id) {
  try {
    const big = BigInt(id);
    if (big < 29700859247n) return null;
    return new Date(Number((big >> 22n) + 1288834974657n));
  } catch {
    return null;
  }
}

/** The account for a tweet; for any other link, the site it is on. */
function who(item) {
  if (item.handle) return `@${item.handle}`;
  if (item.id) return t('links.unknownAuthor');
  try {
    return new URL(item.url).hostname.replace(/^www\./, '');
  } catch {
    return item.url;
  }
}

function stateChip(status) {
  return h('span', { class: `state ${status}` }, icon(status), t(`state.${status}`));
}

/* ----------------------------------------------------------------- state */

const VIEWS = ['overview', 'links', 'download', 'folder', 'settings'];
const ACTIVE = ['running', 'stopping'];

const state = {
  view: 'overview',
  status: null,
  fetchedAt: 0,
  offline: false,
  job: null,
  jobId: 0,
  lines: [],
  shown: 0,
  cursor: 0,
  settings: null,
  draft: {},
  tools: null,
  toolsBusy: false,
  list: { q: '', filter: 'all', items: [], matched: 0, total: 0, busy: false, sig: '' },
  latestSig: '',
  setupSig: '',
  barsSig: '',
  dlLang: '',
  dlCookies: null,
  suggestUpdate: false,
  timer: 0,
  ticking: false,
};

const isActive = (job) => Boolean(job && ACTIVE.includes(job.state));

/* ------------------------------------------------------------------ loop */

async function tick() {
  if (state.ticking) return;
  state.ticking = true;
  clearTimeout(state.timer);
  try {
    await refreshStatus();
    if (!state.offline && (isActive(state.status?.job) || ['download', 'folder', 'settings'].includes(state.view))) {
      await refreshJob();
    }
  } finally {
    state.ticking = false;
    const busy = isActive(state.status?.job);
    state.timer = setTimeout(tick, document.hidden ? 15000 : busy ? 1000 : 2500);
  }
}

/** Refresh now rather than at the next tick, e.g. right after an action. */
function kick() {
  setTimeout(tick, 0);
}

async function refreshStatus() {
  let s;
  try {
    s = await getJSON(`/api/status?strip=1&tz=${new Date().getTimezoneOffset()}`);
  } catch {
    setOffline(true);
    return;
  }
  const before = state.status;
  state.status = s;
  state.fetchedAt = Date.now();
  setOffline(false);
  renderChrome();
  if (before && isActive(before.job) && !isActive(s.job)) jobEnded(s.job, before, s);
  if (state.view === 'overview') renderOverview(before);
  if (state.view === 'links') renderLinksMeta(before);
  if (state.view === 'download') renderDownload();
  if (state.view === 'folder') renderFolder();
  if (state.view === 'settings') renderAbout();
}

function setOffline(offline) {
  if (state.offline === offline) return;
  state.offline = offline;
  $('#offline').hidden = !offline;
  renderChrome();
}

async function refreshJob() {
  let data;
  try {
    data = await getJSON(`/api/job?since=${state.cursor}`);
  } catch {
    return;
  }
  if (data.job && data.job.id !== state.jobId) {
    state.jobId = data.job.id;
    state.lines = [];
    state.shown = 0;
    $('#job-log').replaceChildren();
  }
  state.job = data.job;
  if (data.lines.length) {
    state.lines.push(...data.lines);
    if (state.lines.length > 4000) {
      state.lines.splice(0, state.lines.length - 4000);
      state.shown = 0;
      $('#job-log').replaceChildren();
    }
  }
  state.cursor = Math.max(state.cursor, data.next);
  renderJob();
}

function jobEnded(job, before, after) {
  if (!job) return;
  if (job.kind === 'download' && !job.summary.startsWith('dry run') && !job.summary.startsWith('nothing to')) {
    downloadEnded(job, before.downloads, after.downloads);
  }
  if (job.kind === 'setup') {
    state.suggestUpdate = false;
    loadTools(true);
  }
  if (state.view === 'links') loadLinks(true);
  state.latestSig = '';
  kick();
}

/** What a finished download did, said in the panel and, if allowed, by the system. */
function downloadEnded(job, before, after) {
  const got = Math.max(0, after.done - before.done);
  const failed = Math.max(0, after.failed - before.failed);
  const said = [];
  if (got) said.push(t('notify.got', { n: got }));
  if (failed) said.push(t('notify.failed', { n: failed }));
  const title = t(job.state === 'stopped' ? 'notify.stopped' : 'notify.done');
  const body = said.length ? said.join(', ') : t('notify.nothing');
  toast(`${title}: ${body}`);
  // Nothing came down and several links broke: X has likely changed again.
  if (!got && failed >= 3) {
    state.suggestUpdate = true;
    state.setupSig = '';
  }
  if (document.hidden && 'Notification' in window && Notification.permission === 'granted') {
    try {
      new Notification(title, { body, icon: '/icon.png', tag: 'x-link-collector' });
    } catch {
      // some browsers only allow notifications from a service worker
    }
  }
}

/** Asked once, from the click that starts a download, which is when it makes sense. */
function askToNotify() {
  if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission().catch(() => {});
  }
}

async function startJob(body, message) {
  if (body.kind === 'download' && !body.dry_run) askToNotify();
  try {
    await postJSON('/api/job', body);
    if (message) toast(message);
    await tick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* ---------------------------------------------------------------- chrome */

function applyStatic() {
  document.documentElement.lang = lang;
  for (const node of document.querySelectorAll('[data-t]')) node.textContent = t(node.dataset.t);
  for (const node of document.querySelectorAll('[data-t-placeholder]')) node.placeholder = t(node.dataset.tPlaceholder);
  for (const node of document.querySelectorAll('[data-t-label]')) node.setAttribute('aria-label', t(node.dataset.tLabel));
  $('.tabs').setAttribute('aria-label', t('nav.label'));
  const langButton = $('#lang');
  langButton.textContent = t('lang.name');
  langButton.setAttribute('aria-label', t('lang.label'));
  langButton.lang = lang === 'tr' ? 'en' : 'tr';
  renderThemeButton();
}

function renderChrome() {
  const s = state.status;
  const conn = $('#conn');
  conn.classList.toggle('lost', state.offline);
  $('#conn-text').textContent = state.offline || !s ? t('conn.lost') : t('conn.ok', { port: String(s.port) });
  const restart = $('#restart');
  const want = state.settings?.settings?.port;
  restart.hidden = !(s && s.restart_needed && want);
  if (!restart.hidden) restart.textContent = t('restart', { want: String(want), port: String(s.port) });
  document.title = `${t(`nav.${state.view}`)} – x-link-collector`;
}

function currentTheme() {
  return document.documentElement.dataset.theme
    || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
}

function renderThemeButton() {
  const dark = currentTheme() === 'dark';
  const button = $('#theme');
  button.setAttribute('aria-label', t(dark ? 'theme.toLight' : 'theme.toDark'));
  button.title = button.getAttribute('aria-label');
  button.querySelector('use').setAttribute('href', dark ? '#i-sun' : '#i-moon');
}

function switchTheme() {
  const next = currentTheme() === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = next;
  remember('theme', next);
  renderThemeButton();
  drawStrip();
}

function switchLanguage() {
  lang = lang === 'tr' ? 'en' : 'tr';
  remember('lang', lang);
  makeFormatters();
  applyStatic();
  state.latestSig = '';
  state.setupSig = '';
  state.barsSig = '';
  renderChrome();
  renderView();
}

/* ---------------------------------------------------------------- router */

function route() {
  const wanted = location.hash.slice(1);
  const view = VIEWS.includes(wanted) ? wanted : 'overview';
  const moved = state.view !== view;
  state.view = view;
  for (const name of VIEWS) $(`#view-${name}`).hidden = name !== view;
  for (const link of document.querySelectorAll('.tabs a')) {
    if (link.dataset.view === view) link.setAttribute('aria-current', 'page');
    else link.removeAttribute('aria-current');
  }
  const slot = document.querySelector(`#view-${view} .job-slot`);
  if (slot) slot.append($('#job'));
  renderChrome();
  renderView();
  if (moved) {
    window.scrollTo(0, 0);
    document.querySelector(`#view-${view} h1`)?.focus({ preventScroll: true });
  }
  kick();
}

function renderView() {
  switch (state.view) {
    case 'overview':
      renderOverview(null);
      break;
    case 'links':
      renderFilters();
      loadLinks(true);
      break;
    case 'download':
      renderDownload();
      renderJob();
      break;
    case 'folder':
      renderFolder(true);
      renderJob();
      break;
    case 'settings':
      renderSettings();
      renderTools();
      renderAbout();
      renderJob();
      break;
    default:
      break;
  }
}

/* -------------------------------------------------------------- overview */

function renderOverview(before) {
  const s = state.status;
  if (!s) return;
  const d = s.downloads;
  const job = s.job;
  const downloading = isActive(job) && job.kind === 'download';

  $('#ov-title').textContent = s.total ? t('ov.title', { n: s.total }) : t('ov.titleEmpty');
  let sub;
  if (!s.total) sub = t('ov.subEmpty');
  else if (downloading) sub = t('ov.subRunning');
  else if (d.queued > d.pending && d.pending) sub = t('ov.subRetrying', { n: d.pending, k: d.queued - d.pending });
  else if (d.queued > d.pending) sub = t('ov.subRetryOnly', { n: d.queued });
  else if (d.pending) sub = t('ov.subWaiting', { n: d.pending });
  else if (d.failed && d.no_media) sub = t('ov.subUnfinished', { failed: d.failed, nomedia: d.no_media });
  else if (d.failed) sub = t('ov.subFailed', { n: d.failed });
  else sub = t('ov.subDone');
  if (s.auto_download && s.total && !downloading) sub = `${sub} ${t('ov.subAuto')}`;
  $('#ov-sub').textContent = sub;

  renderGoButton($('#ov-go'), $('#ov-stop'));
  $('#strip-figure').hidden = !s.total;
  if (!before || before.strip !== s.strip || before.total !== s.total) drawStrip();
  renderLegend();
  renderSetup();
  renderBars();
  renderLatest();
}

/** The one action that matters most right now, on the overview and the download page. */
function renderGoButton(go, stop) {
  const s = state.status;
  if (!s) return;
  const d = s.downloads;
  const job = s.job;
  if (stop) stop.hidden = !(isActive(job) && job.kind === 'download');
  go.dataset.retry = '';
  if (isActive(job)) {
    go.disabled = true;
    if (job.kind !== 'download') go.textContent = t('go.busy');
    else if (job.total) go.textContent = t('go.running', { done: job.done, total: job.total });
    else go.textContent = t('go.starting');
  } else if (d.queued) {
    // A plain run: the new links, and failed ones not yet given up on.
    go.disabled = false;
    go.textContent = t('go.download', { n: d.queued });
  } else if (d.failed) {
    // --retry-failed takes the media-less links along, so they are counted.
    go.disabled = false;
    go.dataset.retry = '1';
    go.textContent = t('go.retry', { n: d.failed + d.no_media });
  } else {
    go.disabled = true;
    go.textContent = t('go.nothing');
  }
}

function downloadBody(extra = {}) {
  const body = { kind: 'download', ...extra };
  if (state.view === 'download') {
    if ($('#dl-retry').checked) body.retry_failed = true;
    const limit = parseInt($('#dl-limit').value, 10);
    if (limit > 0) body.limit = limit;
  }
  return body;
}

/* ----------------------------------------------------------------- strip */

/*
 * One square per link, in links.txt order, coloured by what x-download made
 * of it. Drawn on a canvas: a collection of a few thousand links would be a
 * few thousand DOM nodes otherwise. Past what fits, squares stand for groups.
 */
const strip = { cell: 10, gap: 2, cols: 1, cells: 0, group: 1, focus: -1, info: new Map(), pending: 0 };
const STRIP_HEIGHT = 168;
const CODE = { d: 'done', n: 'no-media', f: 'failed', p: 'pending' };

function stripCode(i) {
  const codes = state.status?.strip || '';
  if (strip.group === 1) return codes[i] || 'p';
  const counts = {};
  const end = Math.min(codes.length, (i + 1) * strip.group);
  for (let j = i * strip.group; j < end; j += 1) counts[codes[j]] = (counts[codes[j]] || 0) + 1;
  return Object.keys(counts).sort((a, b) => counts[b] - counts[a])[0] || 'p';
}

function drawStrip() {
  const canvas = $('#strip');
  const codes = state.status?.strip || '';
  const width = $('#strip-area').clientWidth;
  if (!width || state.view !== 'overview') return;

  let group = 1;
  for (;;) {
    let fitted = false;
    for (const cell of [14, 12, 10, 8, 6, 5, 4, 3]) {
      const gap = cell >= 6 ? 2 : 1;
      const cols = Math.max(1, Math.floor((width + gap) / (cell + gap)));
      const cells = Math.ceil(codes.length / group);
      const rows = Math.ceil(cells / cols);
      if (rows * (cell + gap) - gap <= STRIP_HEIGHT) {
        Object.assign(strip, { cell, gap, cols, cells, group, rows });
        fitted = true;
        break;
      }
    }
    if (fitted || group > codes.length) break;
    group *= 2;
  }

  const height = Math.max(0, strip.rows * (strip.cell + strip.gap) - strip.gap);
  const ratio = window.devicePixelRatio || 1;
  canvas.style.height = `${height}px`;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  const ctx = canvas.getContext('2d');
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, width, height);

  const css = getComputedStyle(document.documentElement);
  const colors = {};
  for (const [code, name] of Object.entries(CODE)) colors[code] = css.getPropertyValue(`--${name}`).trim();
  const radius = strip.cell >= 6 ? 2 : 1;
  for (let i = 0; i < strip.cells; i += 1) {
    const x = (i % strip.cols) * (strip.cell + strip.gap);
    const y = Math.floor(i / strip.cols) * (strip.cell + strip.gap);
    ctx.fillStyle = colors[stripCode(i)];
    ctx.beginPath();
    if (ctx.roundRect) ctx.roundRect(x, y, strip.cell, strip.cell, radius);
    else ctx.rect(x, y, strip.cell, strip.cell);
    ctx.fill();
  }
  if (strip.focus >= 0 && strip.focus < strip.cells) {
    const x = (strip.focus % strip.cols) * (strip.cell + strip.gap);
    const y = Math.floor(strip.focus / strip.cols) * (strip.cell + strip.gap);
    ctx.strokeStyle = css.getPropertyValue('--ink').trim();
    ctx.lineWidth = 2;
    ctx.strokeRect(x - 1.5, y - 1.5, strip.cell + 3, strip.cell + 3);
  }

  const d = state.status?.downloads;
  if (d) {
    canvas.setAttribute('aria-label', t('strip.label', {
      total: state.status.total, done: d.done, nomedia: d.no_media, failed: d.failed, waiting: d.pending,
    }));
  }
}

function renderLegend() {
  const d = state.status.downloads;
  const items = [['done', d.done], ['no-media', d.no_media], ['failed', d.failed], ['pending', d.pending]];
  $('#legend').replaceChildren(...items.map(([key, n]) => h('li', {},
    h('span', { class: `swatch ${key}`, 'aria-hidden': 'true' }),
    h('span', { text: t(`state.${key}`) }),
    h('strong', { text: num(n) }))));
  $('#legend-note').textContent = strip.group > 1 ? t('strip.noteGrouped', { k: strip.group }) : t('strip.note');
}

function cellAt(event) {
  const rect = $('#strip').getBoundingClientRect();
  const step = strip.cell + strip.gap;
  const col = Math.floor((event.clientX - rect.left) / step);
  const row = Math.floor((event.clientY - rect.top) / step);
  if (col < 0 || col >= strip.cols || row < 0) return -1;
  const i = row * strip.cols + col;
  return i < strip.cells ? i : -1;
}

function showStripTip(i) {
  const tip = $('#strip-tip');
  if (i < 0) {
    tip.hidden = true;
    return;
  }
  const total = state.status.total;
  const from = i * strip.group + 1;
  const to = Math.min(total, (i + 1) * strip.group);
  const step = strip.cell + strip.gap;
  const lines = [
    h('strong', { text: t(`state.${CODE[stripCode(i)]}`) }),
    h('span', { text: from === to ? t('strip.line', { n: from, total }) : t('strip.lines', { from, to, total }) }),
  ];
  const info = strip.info.get(from);
  if (strip.group === 1 && info) lines.push(h('span', { text: who(info) }));
  tip.replaceChildren(...lines);
  tip.hidden = false;
  const area = $('#strip-area').clientWidth;
  const center = (i % strip.cols) * step + strip.cell / 2;
  const half = tip.offsetWidth / 2;
  tip.style.left = `${Math.min(Math.max(center, half), area - half)}px`;
  tip.style.top = `${Math.floor(i / strip.cols) * step}px`;
  if (strip.group === 1 && !info) fetchLineInfo(from, i);
}

/** Who a square is: one link, fetched when the pointer rests on it. */
function fetchLineInfo(line, cell) {
  clearTimeout(strip.pending);
  strip.pending = setTimeout(async () => {
    try {
      const offset = state.status.total - line;
      const data = await getJSON(`/api/links?offset=${offset}&limit=1`);
      const item = data.items[0];
      if (item && item.line === line) {
        strip.info.set(line, item);
        if (!$('#strip-tip').hidden && strip.hover === cell) showStripTip(cell);
      }
    } catch {
      // the tooltip simply stays without a name
    }
  }, 120);
}

function openCell(i) {
  const info = strip.info.get(i * strip.group + 1);
  if (strip.group !== 1 || !info) return;
  state.list.q = info.id || info.url;
  state.list.filter = 'all';
  $('#links-q').value = state.list.q;
  location.hash = '#links';
}

function wireStrip() {
  const canvas = $('#strip');
  canvas.addEventListener('pointermove', (e) => {
    const i = cellAt(e);
    strip.hover = i;
    showStripTip(i);
  });
  canvas.addEventListener('pointerleave', () => {
    strip.hover = -1;
    if (strip.focus < 0) showStripTip(-1);
  });
  canvas.addEventListener('click', (e) => openCell(cellAt(e)));
  canvas.addEventListener('focus', () => {
    if (strip.focus < 0) strip.focus = 0;
    strip.hover = strip.focus;
    drawStrip();
    showStripTip(strip.focus);
  });
  canvas.addEventListener('blur', () => {
    strip.focus = -1;
    drawStrip();
    showStripTip(-1);
  });
  canvas.addEventListener('keydown', (e) => {
    const moves = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: strip.cols, ArrowUp: -strip.cols };
    let next = strip.focus;
    if (e.key in moves) next += moves[e.key];
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = strip.cells - 1;
    else if (e.key === 'Enter') {
      openCell(strip.focus);
      return;
    } else return;
    e.preventDefault();
    strip.focus = Math.min(Math.max(next, 0), strip.cells - 1);
    strip.hover = strip.focus;
    drawStrip();
    showStripTip(strip.focus);
  });
  new ResizeObserver(() => drawStrip()).observe($('#strip-area'));
}

/* ------------------------------------------------------- per-day columns */

function renderBars() {
  const daily = state.status.daily || [];
  const sig = `${lang}|${daily.join(',')}`;
  if (sig === state.barsSig) return;
  state.barsSig = sig;

  const max = Math.max(0, ...daily);
  const last = daily.length - 1;
  const peak = daily.indexOf(max);
  const today = new Date(serverNow() * 1000);
  const rows = [];
  const slots = daily.map((n, i) => {
    const day = new Date(today);
    day.setDate(today.getDate() - (last - i));
    const label = `${fmt.day.format(day)}: ${t('ov.dayCount', { n })}`;
    rows.push(h('tr', {}, h('th', { scope: 'row', text: fmt.day.format(day) }), h('td', { text: num(n) })));
    const slot = h('div', { class: 'bar-slot', tabindex: '0', 'aria-label': label, 'data-tip': label });
    const height = max ? `${(n / max) * 100}%` : '0%';
    const col = h('div', { class: n ? 'bar-col' : 'bar-col zero' });
    col.style.setProperty('--h', height);
    slot.append(col);
    // Labels only where they say something: today, and the busiest day.
    if (n && (i === last || i === peak)) {
      const value = h('span', { class: 'bar-label', 'aria-hidden': 'true', text: num(n) });
      value.style.setProperty('--h', height);
      slot.append(value);
    }
    return slot;
  });
  $('#bars').replaceChildren(...slots);
  $('#bars-empty').hidden = max > 0;
  $('#bars-table').replaceChildren(h('caption', { text: t('ov.perDay') }), h('tbody', {}, rows));
}

function wireFloatingTips() {
  const tip = $('#tip');
  const show = (target) => {
    const text = target?.dataset?.tip;
    if (!text) {
      tip.hidden = true;
      return;
    }
    tip.textContent = text;
    tip.hidden = false;
    const rect = target.getBoundingClientRect();
    const half = tip.offsetWidth / 2;
    tip.style.left = `${Math.min(Math.max(rect.left + rect.width / 2, half + 8), window.innerWidth - half - 8)}px`;
    tip.style.top = `${rect.top + 12}px`;
  };
  const bars = $('#bars');
  bars.addEventListener('pointerover', (e) => show(e.target.closest('[data-tip]')));
  bars.addEventListener('pointerleave', () => show(null));
  bars.addEventListener('focusin', (e) => show(e.target.closest('[data-tip]')));
  bars.addEventListener('focusout', () => show(null));
}

/* ----------------------------------------------------------- latest/setup */

async function renderLatest() {
  const s = state.status;
  const sig = `${lang}|${s.total}|${s.downloads.done}|${s.downloads.failed}|${s.downloads.no_media}`;
  if (sig === state.latestSig) return;
  state.latestSig = sig;
  try {
    const data = await getJSON('/api/links?limit=6');
    const items = data.items.map((item) => h('li', {},
      h('a', { class: 'handle', href: item.url, target: '_blank', rel: 'noopener noreferrer' },
        who(item), item.id ? h('span', { class: 'id', text: item.id }) : null),
      h('span', { class: 'when', text: item.captured ? ago(item.captured) : '' }),
      stateChip(item.status)));
    $('#latest').replaceChildren(...(items.length ? items : [h('li', { class: 'empty', text: t('ov.latestEmpty') })]));
  } catch {
    state.latestSig = '';
  }
}

function extensionFolder() {
  const dir = state.tools?.tools_dir;
  if (!dir) return null;
  const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/';
  return `${dir.replace(/[\\/]+$/, '')}${sep}extension`;
}

function renderSetup() {
  const s = state.status;
  const tools = state.tools;
  const items = [];
  if (!s.extension_seen) {
    const folder = extensionFolder();
    items.push(h('li', {},
      h('strong', { text: t('setup.ext') }),
      h('p', { text: t('setup.extHow') }),
      folder ? h('div', { class: 'copy-line' },
        h('code', { text: folder }),
        h('button', { type: 'button', class: 'btn small', text: t('copy'), onclick: () => copyText(folder) })) : null));
  }
  if (tools && !tools.python) {
    items.push(h('li', {}, h('strong', { text: t('setup.python') }), h('p', { text: t('setup.pythonHow') })));
  } else if (tools && tools.downloader && !tools.yt_dlp && !tools.gallery_dl) {
    items.push(h('li', {},
      h('strong', { text: t('setup.tools') }),
      h('p', { text: t('setup.toolsHow') }),
      h('div', {}, h('button', {
        type: 'button',
        class: 'btn primary',
        text: t('setup.install'),
        disabled: isActive(s.job),
        onclick: installTools,
      }))));
  }
  // Only an update left to do is upkeep, not setup; the heading says which.
  const setupItems = items.length;
  const age = toolAge(tools?.yt_dlp);
  if (tools?.yt_dlp && (state.suggestUpdate || age > STALE_DAYS)) {
    items.push(h('li', {},
      h('strong', { text: t('setup.update') }),
      h('p', { text: state.suggestUpdate ? t('setup.updateFailing') : t('setup.updateOld', { days: age }) }),
      h('div', {}, h('button', {
        type: 'button',
        class: 'btn primary',
        text: t('setup.updateNow'),
        disabled: isActive(s.job),
        onclick: installTools,
      }))));
  }
  const sig = `${lang}|${!s.extension_seen}|${extensionFolder()}|${tools?.python}|${tools?.yt_dlp}|${tools?.gallery_dl}|${isActive(s.job)}|${state.suggestUpdate}|${age}`;
  if (sig === state.setupSig) return;
  state.setupSig = sig;
  $('#setup').hidden = items.length === 0;
  $('#setup-title').textContent = t(setupItems ? 'setup.title' : 'setup.titleUpkeep');
  $('#setup-items').replaceChildren(...items);
}

/** yt-dlp versions are dates: 2026.08.19. Days since then, or 0 when unknown. */
const STALE_DAYS = 60;
function toolAge(version) {
  const m = /^(\d{4})\.(\d{2})\.(\d{2})/.exec(version || '');
  if (!m) return 0;
  const released = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])) / 1000;
  return Math.max(0, Math.floor((serverNow() - released) / 86400));
}

function installTools() {
  startJob({ kind: 'setup' });
  if (state.view === 'overview') location.hash = '#settings';
}

/* ----------------------------------------------------------------- links */

const FILTERS = ['all', 'pending', 'done', 'no-media', 'failed'];

function renderFilters() {
  const d = state.status?.downloads;
  const counts = d ? { all: state.status.total, pending: d.pending, done: d.done, 'no-media': d.no_media, failed: d.failed } : {};
  $('#links-filter').replaceChildren(...FILTERS.map((key) => h('button', {
    type: 'button',
    'aria-pressed': String(state.list.filter === key),
    onclick: () => {
      state.list.filter = key;
      renderFilters();
      loadLinks(true);
    },
  }, key === 'all' ? t('links.all') : t(`state.${key}`), key in counts ? h('span', { class: 'count', text: num(counts[key]) }) : null)));
  $('#links-file').textContent = state.status?.links_file || '';
}

/** Reload the first page when the numbers move, unless the reader is busy in the list. */
function renderLinksMeta(before) {
  const s = state.status;
  const d = s.downloads;
  const sig = `${s.total}|${d.done}|${d.no_media}|${d.failed}`;
  if (state.list.sig !== sig) {
    renderFilters();
    const interacting = $('#view-links').contains(document.activeElement) && document.activeElement !== document.body
      && !$('#links-filter').contains(document.activeElement);
    if (before && !interacting && state.list.items.length <= 50) loadLinks(true);
  }
}

async function loadLinks(reset) {
  const list = state.list;
  if (list.busy) return;
  list.busy = true;
  const offset = reset ? 0 : list.items.length;
  const query = new URLSearchParams({ q: list.q, status: list.filter, offset: String(offset), limit: '50' });
  try {
    const data = await getJSON(`/api/links?${query}`);
    list.items = reset ? data.items : list.items.concat(data.items);
    list.matched = data.matched;
    list.total = data.total;
    const d = state.status?.downloads;
    if (state.status && d) list.sig = `${state.status.total}|${d.done}|${d.no_media}|${d.failed}`;
    renderLinks();
  } catch (e) {
    toast(explain(e), 'error');
  } finally {
    list.busy = false;
  }
}

function renderLinks() {
  const list = state.list;
  $('#links-list').replaceChildren(...list.items.map(linkRow));
  const empty = $('#links-empty');
  empty.hidden = list.items.length > 0;
  empty.textContent = list.total === 0 ? t('links.empty') : t('links.noMatch');
  $('#links-meta').textContent = list.items.length ? t('links.shown', { shown: list.items.length, matched: list.matched }) : '';
  const more = $('#links-more');
  more.hidden = list.items.length >= list.matched;
  more.textContent = t('links.more');
}

const isVideo = (file) => /\.(mp4|m4v|mov|webm|mkv)$/i.test(file);
const isImage = (file) => /\.(jpe?g|png|webp|gif)$/i.test(file);

/** x-download records paths relative to the media folder, \ on Windows. */
function mediaUrl(file) {
  return `/media/${file.replace(/\\/g, '/').split('/').map(encodeURIComponent).join('/')}`;
}

/** Video thumbnails load as they scroll into view: a page of links is fifty files. */
const lazyVideos = 'IntersectionObserver' in window ? new IntersectionObserver((entries) => {
  for (const entry of entries) {
    if (!entry.isIntersecting) continue;
    entry.target.src = entry.target.dataset.src;
    lazyVideos.unobserve(entry.target);
  }
}, { rootMargin: '200px' }) : null;

function thumb(item) {
  const files = (item.files || []).filter((f) => isVideo(f) || isImage(f));
  if (!files.length) return h('span', { class: 'thumb empty', 'aria-hidden': 'true' });
  const button = h('button', {
    type: 'button',
    class: 'thumb',
    'aria-label': t('links.show'),
    onclick: () => openViewer(item, files, 0),
  });
  const first = files[0];
  if (isImage(first)) {
    button.append(h('img', { src: mediaUrl(first), alt: '', loading: 'lazy', decoding: 'async' }));
  } else {
    // #t= asks for a frame a moment in, rather than a black first one.
    const video = h('video', { muted: true, preload: 'metadata', playsinline: true, 'aria-hidden': 'true', 'data-src': `${mediaUrl(first)}#t=0.5` });
    if (lazyVideos) lazyVideos.observe(video);
    else video.src = video.dataset.src;
    button.append(video, icon('play'));
  }
  if (files.length > 1) button.append(h('span', { class: 'more', text: `+${files.length - 1}` }));
  return button;
}

/* ----------------------------------------------------------------- viewer */

const viewer = { item: null, files: [], index: 0 };

function openViewer(item, files, index) {
  Object.assign(viewer, { item, files, index });
  $('#viewer-title').textContent = who(item);
  const open = $('#viewer-open');
  open.href = item.url;
  open.textContent = t(item.id ? 'links.open' : 'links.openOther');
  showViewerFile();
  $('#viewer').showModal();
}

function showViewerFile() {
  const file = viewer.files[viewer.index];
  $('#viewer-stage').replaceChildren(isVideo(file)
    ? h('video', { src: mediaUrl(file), controls: true, autoplay: true, playsinline: true })
    : h('img', { src: mediaUrl(file), alt: who(viewer.item) }));
  const many = viewer.files.length > 1;
  $('#viewer-count').textContent = many ? t('viewer.count', { i: viewer.index + 1, n: viewer.files.length }) : '';
  $('#viewer-prev').hidden = !many;
  $('#viewer-next').hidden = !many;
}

function stepViewer(by) {
  viewer.index = (viewer.index + by + viewer.files.length) % viewer.files.length;
  showViewerFile();
}

function wireViewer() {
  const dialog = $('#viewer');
  $('#viewer-close').addEventListener('click', () => dialog.close());
  $('#viewer-prev').addEventListener('click', () => stepViewer(-1));
  $('#viewer-next').addEventListener('click', () => stepViewer(1));
  // Closing must also stop a playing video.
  dialog.addEventListener('close', () => $('#viewer-stage').replaceChildren());
  dialog.addEventListener('keydown', (e) => {
    if (viewer.files.length < 2) return;
    if (e.key === 'ArrowLeft') stepViewer(-1);
    if (e.key === 'ArrowRight') stepViewer(1);
  });
  dialog.addEventListener('click', (e) => {
    if (e.target === dialog) dialog.close(); // a click on the dimmed backdrop
  });
}

function downloadOne(item, retry) {
  startJob({ kind: 'download', only: [item.url], retry_failed: retry }, t('job.startedDownload'));
}

function linkRow(item) {
  const posted = item.id ? postedAt(item.id) : null;
  // A row's own next step: fetch what is waiting, retry what did not come down.
  const next = item.status === 'pending' ? 'download' : item.status === 'done' ? null : 'retry';
  return h('li', { class: 'link-row' },
    thumb(item),
    h('span', { class: 'line', text: t('links.line', { n: item.line }) }),
    h('div', { class: 'who' },
      h('a', { class: 'handle', href: item.url, target: '_blank', rel: 'noopener noreferrer' },
        who(item), item.id ? h('span', { class: 'id', text: item.id }) : null),
      h('span', { class: 'url', text: item.url })),
    h('div', { class: 'dates' },
      posted ? h('span', { text: t('links.posted', { date: fmt.date.format(posted) }) }) : null,
      item.captured ? h('span', { text: t('links.collected', { ago: ago(item.captured) }) }) : null),
    stateChip(item.status),
    h('div', { class: 'actions' },
      next ? h('button', {
        type: 'button',
        class: 'btn small quiet',
        text: t(`links.${next}`),
        disabled: isActive(state.status?.job),
        onclick: () => downloadOne(item, next === 'retry'),
      }) : null,
      h('a', { class: 'btn small quiet', href: item.url, target: '_blank', rel: 'noopener noreferrer', text: t(item.id ? 'links.open' : 'links.openOther') }),
      h('button', { type: 'button', class: 'btn small quiet', text: t('links.copy'), onclick: () => copyText(item.url) }),
      h('button', { type: 'button', class: 'btn small quiet danger', text: t('links.remove'), onclick: () => removeLink(item) })),
    item.status === 'failed' && item.error ? h('p', { class: 'why', text: item.error }) : null);
}

async function removeLink(item) {
  const sure = await confirmAction({ title: t('links.removeTitle'), body: `${item.url}\n\n${t('links.removeBody')}`, action: t('links.remove') });
  if (!sure) return;
  try {
    await postJSON('/api/links/remove', { url: item.url });
    toast(t('links.removed'));
    await loadLinks(true);
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

function wireLinks() {
  let typing = 0;
  $('#links-q').addEventListener('input', (e) => {
    clearTimeout(typing);
    typing = setTimeout(() => {
      state.list.q = e.target.value.trim();
      loadLinks(true);
    }, 250);
  });
  const box = $('#links-add');
  const toggle = (open) => {
    box.hidden = !open;
    $('#links-add-open').setAttribute('aria-expanded', String(open));
    if (open) $('#links-add-text').focus();
  };
  $('#links-add-open').addEventListener('click', () => toggle(box.hidden));
  $('#links-add-close').addEventListener('click', () => toggle(false));
  box.addEventListener('submit', async (e) => {
    e.preventDefault();
    const text = $('#links-add-text').value.trim();
    if (!text) return;
    try {
      const r = await postJSON('/api/links', { urls: [text] });
      // Only the parts that happened: "0 were already there" says nothing.
      const said = [];
      if (r.added) said.push(t('links.addedNew', { n: r.added }));
      if (r.duplicate) said.push(t('links.addedKnown', { n: r.duplicate }));
      toast(said.join(', '));
      if (r.rejected) toast(t('links.rejected', { n: r.rejected }), 'error');
      $('#links-add-text').value = '';
      toggle(false);
      await loadLinks(true);
      kick();
    } catch (err) {
      toast(explain(err), 'error');
    }
  });
  $('#links-more').addEventListener('click', () => loadLinks(false));
}

/* -------------------------------------------------------------- download */

const ROUTES = ['off', 'fallback', 'only'];
const BROWSERS = ['brave', 'chrome', 'chromium', 'edge', 'firefox', 'opera', 'safari', 'vivaldi'];

/** The login picker, on the Download page and in Settings: a browser, or a cookies.txt path. */
function fillCookies(select, path, current) {
  const isFile = Boolean(current) && !BROWSERS.includes(current.toLowerCase());
  select.replaceChildren(
    h('option', { value: '', text: t('cookies.none') }),
    ...BROWSERS.map((b) => h('option', { value: b, text: b[0].toUpperCase() + b.slice(1) })),
    h('option', { value: '@file', text: t('cookies.file') }));
  select.value = isFile ? '@file' : (current || '').toLowerCase();
  path.hidden = !isFile;
  path.value = isFile ? current : '';
}

/*
 * Polled every couple of seconds, so it only touches what changed: rebuilding
 * a button or a select under the reader would take their focus away.
 */
function renderDownload() {
  renderGoButton($('#dl-go'));
  const s = state.settings?.settings;
  if (!s) return;
  const active = document.activeElement;

  if (state.dlLang !== lang) {
    state.dlLang = lang;
    state.dlCookies = null;
    $('#dl-route').replaceChildren(...ROUTES.map((key) => h('button', {
      type: 'button',
      'data-route': key,
      onclick: () => saveNow({ mirror: key }),
    }, t(`route.${key}`))));
  }
  for (const button of $('#dl-route').children) {
    button.setAttribute('aria-pressed', String(button.dataset.route === s.mirror));
  }
  $('#dl-route-help').textContent = t(`route.${s.mirror}Help`);

  if (active !== $('#dl-jobs')) $('#dl-jobs').value = s.jobs;

  const select = $('#dl-cookies');
  const path = $('#dl-cookies-path');
  const cookies = `${lang}|${s.cookies}`;
  if (state.dlCookies !== cookies && active !== select && active !== path) {
    state.dlCookies = cookies;
    fillCookies(select, path, s.cookies);
  }

  $('#dl-auto').checked = Boolean(s.auto_download);
}

async function saveNow(changes) {
  try {
    state.settings = await postJSON('/api/settings', changes);
    for (const key of Object.keys(changes)) delete state.draft[key];
    toast(t('dl.saved'));
    renderView();
    kick();
  } catch (e) {
    toast(explain(e), 'error');
    renderView();
  }
}

function wireDownload() {
  const jobsInput = $('#dl-jobs');
  const setJobs = (n) => {
    const value = Math.min(10, Math.max(1, n));
    jobsInput.value = value;
    if (value !== state.settings?.settings?.jobs) saveNow({ jobs: value });
  };
  $('#dl-jobs-less').addEventListener('click', () => setJobs(parseInt(jobsInput.value, 10) - 1));
  $('#dl-jobs-more').addEventListener('click', () => setJobs(parseInt(jobsInput.value, 10) + 1));
  jobsInput.addEventListener('change', () => setJobs(parseInt(jobsInput.value, 10) || 1));

  $('#dl-cookies').addEventListener('change', (e) => {
    const value = e.target.value;
    if (value === '@file') {
      $('#dl-cookies-path').hidden = false;
      $('#dl-cookies-path').focus();
      return;
    }
    saveNow({ cookies: value });
  });
  $('#dl-cookies-path').addEventListener('change', (e) => saveNow({ cookies: e.target.value.trim() }));
  $('#dl-auto').addEventListener('change', (e) => saveNow({ auto_download: e.target.checked }));

  $('#dl-go').addEventListener('click', (e) => {
    startJob(downloadBody(e.currentTarget.dataset.retry ? { retry_failed: true } : {}), t('job.startedDownload'));
  });
  $('#dl-preview').addEventListener('click', () => startJob(downloadBody({ dry_run: true }), t('job.startedPreview')));
  $('#ov-go').addEventListener('click', (e) => {
    startJob({ kind: 'download', retry_failed: Boolean(e.currentTarget.dataset.retry) }, t('job.startedDownload'));
  });
  $('#ov-stop').addEventListener('click', stopJob);
  $('#job-stop').addEventListener('click', stopJob);
}

async function stopJob() {
  try {
    await postJSON('/api/job/stop');
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* -------------------------------------------------------------- job panel */

function renderJob() {
  const panel = $('#job');
  // Shown on the pages that start jobs; the overview has its own progress.
  panel.hidden = !document.querySelector(`#view-${state.view} .job-slot`);
  if (panel.hidden) return;
  const job = state.job;
  const log = $('#job-log');

  if (!job) {
    $('#job-title').textContent = t('job.output');
    $('#job-meta').textContent = '';
    $('#job-stop').hidden = true;
    $('#job-progress').hidden = true;
    if (!log.childElementCount) log.append(h('span', { class: 'idle', text: t('job.idle') }));
    return;
  }

  $('#job-title').textContent = t(`job.title.${job.kind}`);
  const active = isActive(job);
  $('#job-stop').hidden = !active;
  $('#job-stop').disabled = job.state === 'stopping';

  const parts = [t(`job.state.${job.state}`)];
  if (job.total) parts.push(t('job.progress', { done: job.done, total: job.total }));
  if (active && job.total && job.done) {
    const elapsed = serverNow() - job.started;
    parts.push(t('job.left', { t: duration((elapsed / job.done) * (job.total - job.done)) }));
  }
  // How it ended is the log's last line, just below.
  $('#job-meta').textContent = parts.join(', ');

  const progress = $('#job-progress');
  progress.hidden = !active && !job.total;
  progress.classList.toggle('busy', active && !job.total);
  const share = job.total ? (job.done / job.total) * 100 : job.state === 'done' ? 100 : 0;
  $('#job-fill').style.setProperty('--p', `${share}%`);

  const idle = log.querySelector('.idle');
  if (idle) idle.remove();
  if (state.shown < state.lines.length) {
    const atBottom = log.scrollTop + log.clientHeight >= log.scrollHeight - 32;
    const batch = document.createDocumentFragment();
    for (const line of state.lines.slice(state.shown)) batch.append(h('span', { class: lineKind(line), text: `${lineText(line)}\n` }));
    state.shown = state.lines.length;
    log.append(batch);
    if (atBottom) log.scrollTop = log.scrollHeight;
  }
}

/** The receiver's own closing line, in the reader's language; the tools' output stays as they wrote it. */
function lineText(line) {
  const end = line.text.startsWith('-- ') ? line.text.slice(3) : null;
  if (end === 'finished') return `-- ${t('job.state.done')}`;
  if (end === 'stopped') return `-- ${t('job.state.stopped')}`;
  const exited = end && /^exited with code (-?\d+)$/.exec(end);
  if (exited) return `-- ${t('job.exited', { code: exited[1] })}`;
  return line.text;
}

function lineKind(line) {
  if (line.err) return 'err';
  if (line.text.startsWith('$ ')) return 'cmd';
  if (line.text.startsWith('-- ')) return 'end';
  if (/^\[\s*\d+\/\d+\]\s+ok\b/.test(line.text)) return 'ok';
  return '';
}

/* ---------------------------------------------------------------- folder */

function renderFolder(first) {
  const s = state.status;
  const view = state.settings;
  if (!s) return;
  $('#fo-path').textContent = s.media_dir;
  const m = s.media;
  const count = $('#fo-count');
  if (m.files) {
    count.replaceChildren(
      t('fo.files', { n: m.files, size: bytes(m.bytes) }),
      h('span', { class: 'sub', text: t('fo.kinds', { videos: m.videos, images: m.images }) }));
  } else {
    count.textContent = t('fo.none');
  }
  $('#fo-undo').hidden = !m.flattened;
  const busy = isActive(s.job);
  for (const id of ['#fo-preview', '#fo-go', '#fo-undo']) $(id).disabled = busy;
  if (first && view) {
    const set = view.settings;
    $('#fo-dest').value = set.flatten_dest;
    $('#fo-dest').placeholder = view.effective.flatten_dest;
    $('#fo-prefix').checked = set.flatten_prefix_handle;
    $('#fo-videos').checked = set.flatten_videos_only;
    $('#fo-copy').checked = set.flatten_copy;
  }
}

function flattenBody(extra) {
  return {
    kind: 'flatten',
    dest: $('#fo-dest').value.trim(),
    prefix_handle: $('#fo-prefix').checked,
    videos_only: $('#fo-videos').checked,
    copy: $('#fo-copy').checked,
    ...extra,
  };
}

function wireFolder() {
  $('#fo-open').addEventListener('click', () => openThing('media'));
  $('#fo-preview').addEventListener('click', () => startJob(flattenBody({ dry_run: true })));
  $('#fo-go').addEventListener('click', async () => {
    const dest = $('#fo-dest').value.trim() || $('#fo-dest').placeholder;
    const sure = await confirmAction({ title: t('fo.flattenTitle'), body: t('fo.flattenBody', { dest }), action: t('fo.flatten') });
    if (sure) startJob(flattenBody({}));
  });
  $('#fo-undo').addEventListener('click', async () => {
    const sure = await confirmAction({ title: t('fo.undoTitle'), body: t('fo.undoBody'), action: t('fo.undo') });
    if (sure) startJob({ kind: 'unflatten', dest: $('#fo-dest').value.trim() });
  });
}

async function openThing(what) {
  try {
    await postJSON('/api/open', { what });
  } catch (e) {
    toast(explain(e), 'error');
  }
}

/* -------------------------------------------------------------- settings */

/*
 * The form is built from this list. Values are edited into `state.draft` and
 * saved together; settings fixed on the command line show as read-only.
 */
const FORM = [
  ['st.files', [
    { key: 'links_file', kind: 'path', placeholder: (v) => v.defaults.links_file, open: 'links' },
    { key: 'media_dir', kind: 'path', placeholder: (v) => v.defaults.media_dir, open: 'media' },
  ]],
  ['st.downloading', [
    { key: 'mirror', kind: 'choice', options: ROUTES, label: 'st.mirror', optionLabel: (k) => t(`route.${k}`), help: (v) => t(`route.${v}Help`) },
    { key: 'jobs', kind: 'number', min: 1, max: 10, label: 'st.jobs', help: () => t('dl.jobsHelp') },
    { key: 'cookies', kind: 'cookies', label: 'st.cookies', help: () => t('dl.cookiesHelp') },
    { key: 'timeout', kind: 'number', min: 30, max: 86400, unit: 'st.seconds' },
    { key: 'metadata', kind: 'switch' },
    { key: 'auto_download', kind: 'switch', help: () => t('dl.autoHelp') },
  ]],
  ['st.flatten', [
    { key: 'flatten_dest', kind: 'path', placeholder: (v) => v.effective.flatten_dest },
    { key: 'flatten_prefix_handle', kind: 'switch' },
    { key: 'flatten_videos_only', kind: 'switch' },
    { key: 'flatten_copy', kind: 'switch' },
  ]],
  ['st.collector', [
    { key: 'autostart', kind: 'autostart' },
    { key: 'port', kind: 'number', min: 1024, max: 65535 },
    { key: 'fsync', kind: 'switch' },
  ]],
  ['st.where', [
    { key: 'tools_dir', kind: 'path', placeholder: () => state.tools?.tools_dir || '', open: 'tools' },
    { key: 'python', kind: 'path', placeholder: () => state.tools?.python || '' },
  ]],
];

async function loadSettings() {
  try {
    state.settings = await getJSON('/api/settings');
    renderChrome();
    renderView();
  } catch {
    // the status loop shows the panel is offline
  }
}

function value(key) {
  return key in state.draft ? state.draft[key] : state.settings.settings[key];
}

function setDraft(key, raw) {
  const saved = state.settings.settings[key];
  if (raw === saved) delete state.draft[key];
  else state.draft[key] = raw;
  $('#savebar').hidden = Object.keys(state.draft).length === 0;
}

function renderSettings() {
  const view = state.settings;
  if (!view) return;
  const form = $('#settings-form');
  if (form.contains(document.activeElement) && document.activeElement !== form) return;
  const pinned = new Set(view.pinned || []);

  form.replaceChildren(...FORM.map(([title, fields]) => h('section', { class: 'settings-group' },
    h('h2', { text: t(title) }),
    // Starting at sign-in is offered only where the receiver can arrange it.
    fields
      .filter((field) => field.kind !== 'autostart' || typeof view.autostart === 'boolean')
      .map((field) => settingRow(field, view, pinned.has(field.key))))));
  $('#savebar').hidden = Object.keys(state.draft).length === 0;
}

function settingRow(field, view, pinned) {
  const id = `set-${field.key}`;
  const labelKey = field.label || `st.${field.key}`;
  const helpText = field.help ? field.help(value(field.key)) : WORDS.en[`st.${field.key}Help`] ? t(`st.${field.key}Help`) : '';
  const help = helpText ? h('p', { class: 'help', id: `${id}-help`, text: helpText }) : null;
  const described = help ? `${id}-help` : null;
  let label = h('label', { class: 'label', for: id, text: t(labelKey) });
  let control;

  if (field.kind === 'autostart') {
    // Not part of the settings file: it is applied the moment it is switched.
    const input = h('input', { type: 'checkbox', role: 'switch', id, 'aria-describedby': described });
    input.checked = view.autostart;
    input.addEventListener('change', async () => {
      try {
        await postJSON('/api/autostart', { on: input.checked });
        state.settings.autostart = input.checked;
        toast(t('dl.saved'));
      } catch (e) {
        input.checked = !input.checked;
        toast(explain(e), 'error');
      }
    });
    control = h('label', { class: 'switch' }, input, h('span', { class: 'knob', 'aria-hidden': 'true' }));
  } else if (field.kind === 'switch') {
    const input = h('input', { type: 'checkbox', role: 'switch', id, disabled: pinned, 'aria-describedby': described });
    input.checked = Boolean(value(field.key));
    input.addEventListener('change', () => setDraft(field.key, input.checked));
    control = h('label', { class: 'switch' }, input, h('span', { class: 'knob', 'aria-hidden': 'true' }));
  } else if (field.kind === 'choice') {
    // Updated in place: re-rendering would take focus from the button just pressed.
    label = h('span', { class: 'label', id: `${id}-label`, text: t(labelKey) });
    control = h('div', { class: 'segmented', role: 'group', 'aria-labelledby': `${id}-label` });
    for (const option of field.options) {
      control.append(h('button', {
        type: 'button',
        'data-option': option,
        'aria-pressed': String(value(field.key) === option),
        disabled: pinned,
        onclick: () => {
          setDraft(field.key, option);
          for (const button of control.children) {
            button.setAttribute('aria-pressed', String(button.dataset.option === option));
          }
          if (help && field.help) help.textContent = field.help(option);
        },
      }, field.optionLabel(option)));
    }
  } else if (field.kind === 'cookies') {
    const select = h('select', { class: 'input', id, disabled: pinned, 'aria-describedby': described });
    const path = h('input', {
      class: 'input mono',
      type: 'text',
      spellcheck: 'false',
      autocomplete: 'off',
      disabled: pinned,
      placeholder: t('dl.cookiesPath'),
      'aria-label': t('dl.cookiesPath'),
    });
    fillCookies(select, path, value(field.key));
    select.addEventListener('change', () => {
      const file = select.value === '@file';
      path.hidden = !file;
      if (file) path.focus();
      setDraft(field.key, file ? path.value.trim() : select.value);
    });
    path.addEventListener('input', () => setDraft(field.key, path.value.trim()));
    control = h('div', { class: 'stack' }, select, path);
  } else if (field.kind === 'number') {
    const input = h('input', { class: 'input', type: 'number', id, min: field.min, max: field.max, inputmode: 'numeric', disabled: pinned, 'aria-describedby': described });
    input.value = value(field.key);
    input.addEventListener('input', () => {
      const n = Number(input.value);
      if (Number.isInteger(n) && n >= field.min && n <= field.max) setDraft(field.key, n);
    });
    control = field.unit ? h('div', { class: 'number-unit' }, input, h('span', { text: t(field.unit) })) : input;
  } else {
    const input = h('input', {
      class: field.kind === 'path' ? 'input mono' : 'input',
      type: 'text',
      id,
      spellcheck: 'false',
      autocomplete: 'off',
      disabled: pinned,
      placeholder: field.placeholder ? field.placeholder(view) : '',
      'aria-describedby': described,
    });
    input.value = value(field.key);
    input.addEventListener('input', () => setDraft(field.key, input.value.trim()));
    control = field.open
      ? h('div', { class: 'with-button' }, input, h('button', { type: 'button', class: 'btn', text: t('st.openFolder'), onclick: () => openThing(field.open) }))
      : input;
  }

  return h('div', { class: 'setting' },
    h('div', { class: 'about' }, label, help),
    h('div', { class: 'control' }, control, pinned ? h('p', { class: 'pinned', text: t('st.pinned') }) : null));
}

async function saveSettings() {
  try {
    state.settings = await postJSON('/api/settings', state.draft);
    state.draft = {};
    $('#savebar').hidden = true;
    toast(t('save.saved'));
    document.activeElement?.blur();
    renderView();
    renderChrome();
    kick();
  } catch (e) {
    toast(explain(e), 'error');
  }
}

function wireSettings() {
  $('#save-go').addEventListener('click', saveSettings);
  $('#save-discard').addEventListener('click', () => {
    state.draft = {};
    $('#savebar').hidden = true;
    document.activeElement?.blur();
    renderSettings();
  });
  $('#settings-form').addEventListener('submit', (e) => {
    e.preventDefault();
    if (Object.keys(state.draft).length) saveSettings();
  });
  $('#tools-check').addEventListener('click', () => loadTools(true));
  $('#quit').addEventListener('click', async () => {
    const sure = await confirmAction({ title: t('st.quitTitle'), body: t('st.quitHelp'), action: t('st.quit') });
    if (!sure) return;
    try {
      await postJSON('/api/quit');
      toast(t('st.quitDone'));
    } catch (e) {
      toast(explain(e), 'error');
    }
  });
  $('#tools-install').addEventListener('click', installTools);
}

async function loadTools(refresh) {
  if (state.toolsBusy) return;
  state.toolsBusy = true;
  renderTools();
  try {
    state.tools = await getJSON(`/api/tools${refresh ? '?refresh=1' : ''}`);
  } catch (e) {
    if (refresh) toast(explain(e), 'error');
  } finally {
    state.toolsBusy = false;
    state.setupSig = '';
    renderTools();
    if (state.view === 'overview' && state.status) renderSetup();
    if (state.view === 'settings') renderSettings();
  }
}

function renderTools() {
  const facts = $('#tools-facts');
  const tools = state.tools;
  $('#tools-check').disabled = state.toolsBusy;
  $('#tools-install').disabled = !tools?.downloader || isActive(state.status?.job);
  if (state.toolsBusy && !tools) {
    facts.replaceChildren(h('dt', { text: t('st.checking') }), h('dd'));
    return;
  }
  if (!tools) return;
  const missing = (text) => h('dd', { class: 'missing', text });
  const rows = [
    ['Python', tools.python_version ? h('dd', { text: `${tools.python_version} (${tools.python})` }) : missing(t('st.notFound'))],
    ['yt-dlp', tools.yt_dlp
      ? h('dd', { text: toolAge(tools.yt_dlp) ? `${tools.yt_dlp} (${t('st.released', { ago: ago(serverNow() - toolAge(tools.yt_dlp) * 86400) })})` : tools.yt_dlp })
      : missing(t('st.missing'))],
    ['gallery-dl', tools.gallery_dl ? h('dd', { text: tools.gallery_dl }) : missing(t('st.missing'))],
    ['ffmpeg', tools.ffmpeg ? h('dd', { text: tools.ffmpeg }) : missing(t('st.ffmpegMissing'))],
  ];
  if (tools.yt_dlp || tools.gallery_dl) {
    rows.push(['', h('dd', { class: 'help', text: t('st.found', { where: tools.venv ? t('st.venv') : t('st.path') }) })]);
  }
  if (tools.error) rows.push(['', missing(tools.error)]);
  facts.replaceChildren(...rows.flatMap(([name, dd]) => [h('dt', { text: name }), dd]));
}

function renderAbout() {
  const s = state.status;
  const view = state.settings;
  if (!s) return;
  const link = h('a', { href: 'https://github.com/abel0x/x-link-collector', target: '_blank', rel: 'noopener noreferrer', text: 'github.com/abel0x/x-link-collector' });
  const rows = [
    [t('st.version'), h('dd', { text: s.version })],
    [t('st.linksFileNow'), h('dd', { class: 'mono', text: s.links_file })],
  ];
  if (view) {
    rows.push([t('st.configFile'), h('dd', {},
      h('span', { class: 'mono', text: view.config_file }), ' ',
      h('button', { type: 'button', class: 'btn small quiet', text: t('st.openFolder'), onclick: () => openThing('config') }))]);
  }
  rows.push([t('st.source'), h('dd', {}, link)]);
  $('#about-facts').replaceChildren(...rows.flatMap(([name, dd]) => [h('dt', { text: name }), dd]));
}

/* ------------------------------------------------------------------ start */

function start() {
  const saved = remember('theme');
  if (saved === 'dark' || saved === 'light') document.documentElement.dataset.theme = saved;
  makeFormatters();
  applyStatic();

  $('#lang').addEventListener('click', switchLanguage);
  $('#theme').addEventListener('click', switchTheme);
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    renderThemeButton();
    drawStrip();
  });
  window.addEventListener('hashchange', route);
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) kick();
  });
  window.addEventListener('beforeunload', (e) => {
    if (Object.keys(state.draft).length) e.preventDefault();
  });

  wireStrip();
  wireFloatingTips();
  wireLinks();
  wireDownload();
  wireFolder();
  wireSettings();
  wireViewer();

  route();
  loadSettings();
  loadTools(false);
}

start();
