# Image Sources — venuscollect

Research on where to source art images. Sources are ranked by fit for
the project (portrait / feminine / botanical / classical aesthetic).

---

## ✅ Already Integrated

| Source | Type | License | API key needed | Rate limit |
|---|---|---|---|---|
| **Met Museum** | Classical art | Public domain | No | ~80 req/s |
| **AIC** (Art Institute of Chicago) | Classical art | Public domain | No | Reasonable |
| **Unsplash** | Photography | Unsplash License* | Yes (in .env) | 50 req/hr (demo) |

\* Unsplash License = free use, attribution appreciated. Not strictly public domain.
Apply for Production access at https://unsplash.com/developers to raise the limit to 5,000 req/hr.

---

## 🔑 High Priority — Easy to Add

### Rijksmuseum (Dutch National Museum)
- **Best for:** Dutch Golden Age, Vermeer, Rembrandt, botanical still-lifes, portraits
- **License:** CC0 / Public Domain
- **API key:** Free — register at https://data.rijksmuseum.nl/user-generated-content/api/
- **Endpoint:** `GET https://www.rijksmuseum.nl/api/en/collection?q={query}&apikey={key}&imgonly=True`
- **Rate limit:** Not published — very lenient in practice
- **Verdict:** 🌟 Top pick. Huge collection of exactly the kind of art this project targets.

### Wikimedia Commons
- **Best for:** Anything public domain — old masters, Art Nouveau prints, mythology paintings
- **License:** Various (filter for CC0 / PD)
- **API key:** None needed
- **Endpoint:** `https://commons.wikimedia.org/w/api.php?action=query&generator=search&gsrsearch={query}&prop=imageinfo&iiprop=url|mime`
- **Rate limit:** Polite usage expected; add a User-Agent header
- **Verdict:** 🌟 Largest freely available art image repo. Slightly messier metadata.

### Europeana
- **Best for:** European cultural heritage — Renaissance, Baroque, Art Nouveau, mythology
- **License:** CC0 / Public Domain (filterable)
- **API key:** Free — register at https://apis.europeana.eu/
- **Endpoint:** `GET https://api.europeana.eu/record/v2/search.json?query={q}&reusability=open&media=true&wskey={key}`
- **Rate limit:** 10,000 calls/day on free tier
- **Verdict:** Great depth of European classical art, especially French and Italian.

---

## 📸 Photography — Nice to Have

### Flickr
- **Best for:** Vintage photos, Creative Commons art photography, scanned prints
- **License:** Must filter — use `license=4,5,7,9,10` (CC BY / CC0 / Public Domain)
- **API key:** Free — register at https://www.flickr.com/services/apps/create/
- **Endpoint:** `GET https://www.flickr.com/services/rest/?method=flickr.photos.search&api_key={key}&text={query}&license=4,5,7,9,10&media=photos&format=json`
- **Rate limit:** 3,600 req/hr
- **Note:** Quality varies widely — worth adding a Claude review step to filter
- **Verdict:** Good supplementary volume. Needs extra filtering.

---

## 🏛️ Museum APIs — Lower Priority (Smaller / More Niche)

### Smithsonian Open Access
- **Best for:** American art, natural history illustrations, botanical drawings
- **License:** CC0
- **API key:** Free — https://edan.si.edu/openaccess/apidocs/
- **Endpoint:** `GET https://api.si.edu/openaccess/api/v1.0/search?q={query}&api_key={key}&rows=100`
- **Rate limit:** 1,000 calls/day on free tier

### Harvard Art Museums
- **Best for:** Classical antiquities, European paintings, prints
- **License:** CC0 for images where rights are cleared
- **API key:** Free — https://github.com/harvardartmuseums/api-docs
- **Endpoint:** `GET https://api.harvardartmuseums.org/object?q={query}&apikey={key}&hasimage=1`
- **Rate limit:** 2,500 calls/day

### Library of Congress (loc.gov)
- **Best for:** American prints, photographs, vintage illustrations
- **License:** Public Domain
- **API key:** None
- **Endpoint:** `GET https://www.loc.gov/photos/?q={query}&fo=json`
- **Rate limit:** None published — be polite

### NYPL Digital Collections
- **Best for:** Vintage illustrations, botanical prints, New York history
- **License:** Public Domain
- **API key:** Free — https://api.repo.nypl.org/
- **Endpoint:** `GET https://api.repo.nypl.org/api/v2/items/search?q={query}&publicDomainOnly=true`

---

## Recommended Next Steps

1. **Rijksmuseum** — register for a free key (5 min) and add it to `.env` as `RIJKSMUSEUM_API_KEY`. Highest quality match for the project aesthetic.
2. **Wikimedia Commons** — no key needed; add after Rijksmuseum since metadata parsing is slightly more complex.
3. **Europeana** — good after the above two are stable; requires a free API key.
4. **Flickr** — add last; the extra filtering logic (license + Claude review) makes it more work per photo.

---

## `.env` keys to add

```env
RIJKSMUSEUM_API_KEY=your_key_here
EUROPEANA_API_KEY=your_key_here
FLICKR_API_KEY=your_key_here
```
