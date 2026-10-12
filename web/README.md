# Web de Shard

Landing estática (estilo Grafito) para `shardlauncher.com`, servida por un Worker de Cloudflare solo con assets (`wrangler.jsonc`).

- Vista previa local: `python3 -m http.server 8787 --directory web/public`.
- Publicar: `cd web && npx wrangler login && npx wrangler deploy`. Necesita el dominio en la cuenta de Cloudflare.
- Las capturas salen de los goldens de la app (`app/test/goldens/*.png`, convertidas a WebP).
- Las descargas apuntan a `dl.shardlauncher.com/linux/` (bucket R2, ver `docs/hosting.md`).
