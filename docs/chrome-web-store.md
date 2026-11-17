# Publishing the extension on the Chrome Web Store

Chromium users otherwise load the extension unpacked, with Developer mode on.
A Chrome Web Store listing, even an unlisted one, makes that one click, and the
store keeps the extension up to date by itself.

## The id has to stay the same

x-link-receiver takes links only from the extension it knows: in Chromium, by
id (`jnedknmekfknjaajolahghmpigmmfell`). That id follows from the public `key`
in `extension/manifest.json`, so an unpacked copy has it in every Chromium
browser. The store must give the extension the same one, and it does when the
**first** upload carries the private key as `key.pem`:

```sh
make store-zip FIRST=1    # first upload only: adds key.pem
make store-zip            # every upload after that
```

Both write `dist/x-link-collector-chrome-web-store.zip`, with the manifest's
`key` left out (the store does not take it). The private key is
`~/.local/share/x-link-collector-signing/chrome-extension.pem` (set
`STORE_KEY` to use another). Keep a copy somewhere safe, never in the
repository: without it a new id would have to be added to the receiver.

After the first upload, check the item's id in the developer dashboard. If it
is anything other than the one above, add it to `CHROMIUM_IDS` in
`receiver/src/http.rs` (and to the test in `tests/extension.test.mjs`) and
release; until then, `X_LINK_EXTENSION_IDS=<id>` lets a receiver take it.

## Steps

1. Register at <https://chrome.google.com/webstore/devconsole> (a one-time
   fee), then **New item** and upload the zip from `make store-zip FIRST=1`.
2. **Store listing:** the texts below, the 128 px icon from
   `extension/icons/icon-128.png`, and at least one 1280×800 screenshot (the
   panel, say).
3. **Privacy:** the single purpose and the permission reasons below; under data
   use, nothing is collected.
4. **Distribution:** *Unlisted* keeps it off search, reachable by its link;
   *Public* lists it.
5. Submit for review. Later versions: `make store-zip`, then **Upload new
   package** on the same item. The version in the manifest must go up each
   time, as it already does for every release.

## Listing

### English (default)

**Name:** X Link Collector

**Summary:** Middle-click a tweet: the link is saved to a file on your computer
and the tab closes.

**Description:**

> X Link Collector files the tweets you open into links.txt on your own
> computer and closes their tabs, so a reading session leaves a list behind
> instead of fifty tabs.
>
> It works with x-link-receiver, a small program that runs on your machine:
> the extension hands each link to it over 127.0.0.1 and to nothing else.
> The receiver's panel, in your browser, then downloads every photo and video
> at full quality, shows them in a gallery, and exports the list.
>
> - Middle-click a tweet, or use the right-click menu.
> - A tab closes only once its link is on disk; if the receiver is not
>   running, tabs stay open and nothing is lost.
> - No account, no server, no analytics. Nothing leaves your computer.
>
> x-link-receiver and the source: https://github.com/abel0x/x-link-collector

### Türkçe

**Ad:** X Link Collector

**Özet:** Bir tweet’e orta tıklayın: bağlantı bilgisayarınızdaki bir dosyaya
kaydedilir, sekme kapanır.

**Açıklama:**

> X Link Collector, açtığınız tweet’leri kendi bilgisayarınızdaki links.txt
> dosyasına kaydeder ve sekmelerini kapatır; böylece bir okuma oturumunun
> ardından elli sekme değil, bir liste kalır.
>
> Bilgisayarınızda çalışan küçük bir program olan x-link-receiver ile
> birlikte çalışır: eklenti her bağlantıyı yalnızca 127.0.0.1 üzerinden ona
> iletir. Alıcının tarayıcıda açılan paneli de tüm fotoğraf ve videoları en
> yüksek kalitede indirir, bir galeride gösterir ve listeyi dışa aktarır.
>
> - Bir tweet’e orta tıklayın ya da sağ tık menüsünü kullanın.
> - Sekme ancak bağlantısı diske yazıldıktan sonra kapanır; alıcı çalışmıyorsa
>   sekmeler açık kalır, hiçbir şey kaybolmaz.
> - Hesap yok, sunucu yok, analitik yok. Hiçbir şey bilgisayarınızdan çıkmaz.
>
> x-link-receiver ve kaynak kod: https://github.com/abel0x/x-link-collector

## Privacy practices

**Single purpose:** save the tweet links the user opens or picks to a file on
their own computer, through x-link-receiver running there.

| Permission | Why |
|---|---|
| `tabs` | To see a tab's address when it opens on a tweet, and to close that tab once its link is saved. |
| `storage` | To remember the receiver's port, if the user changes it. |
| `contextMenus` | For the "Collect this tweet" item in the right-click menu. |
| `http://127.0.0.1/*` | To hand the link to x-link-receiver on the user's own computer. No other host is contacted. |

**Remote code:** none; every script is in the package.

**Data use:** the extension collects no user data. Links go to a program on
the user's own computer and nowhere else.
