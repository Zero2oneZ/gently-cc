# /social — Surface Social Protocols
<!-- grim_hash: sha256:{grim:social} | orc: worg | kind: integration -->

Access Discord, Telegram, TikTok, Instagram, WhatsApp, X from inside Claude Code.
All actions are capability-gated (Sui scope). Requires trollz-mcp running.

## Triggers
- `/social` — show connected platforms and capabilities
- `/social discord <message>` — send to Discord
- `/social telegram <message>` — send to Telegram
- `/social post <platform> <content>` — post to platform
- `/social search <query>` — web search via Serper
- `/social leads @handle` — extract Instagram leads
- `/social ads <product>` — generate 6 ad concepts
- `discord: <message>` — shorthand for Discord send
- `telegram: <message>` — shorthand for Telegram send

## Platform Capabilities

### Discord (capability: `discord`)
Tool: `trollz_discord_send`
```json
{ "channel_id": "...", "content": "...", "embed": {...} }
```
- Supports: text, embeds, files, mentions
- Env: `DISCORD_BOT_TOKEN`, `DISCORD_CHANNEL_ID`

### Telegram (capability: `telegram`)
Tool: `trollz_telegram_send`
```json
{ "chat_id": "...", "text": "...", "photo_url": "...", "thread_id": "..." }
```
- Supports: text, photos, thread replies
- Env: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`

### Instagram Analytics (capability: `instagram`)
Tools: `trollz_instagram_engagement`, `trollz_instagram_leads`
- Engagement: likes, comments, reach, saves per post
- Leads: commenters → lead list with handle + engagement score
- Read-only (no write capability)

### Web Search (public — no capability required)
Tool: `trollz_web_search`
```json
{ "query": "...", "num": 10 }
```
- Powered by Google Serper
- Returns: title, url, snippet, date

### Ad Generation (capability: `social:post`)
Tool: `trollz_generate_ads`
```json
{ "product": "...", "audience": "...", "platform": "tiktok|instagram|x" }
```
- Returns 6 hook-first ad concepts with copy variants

### TikTok
Currently via `trollz_web_search` + `trollz_generate_ads` pipeline.
Direct TikTok API: roadmap — add `tiktok` capability to scope.

### WhatsApp (capability: `whatsapp`)
Tool: `trollz_whatsapp_send`
- Requires WHATSAPP_BRIDGE_URL env
- Text + media

## Status check
`/social` with no args calls `trollz_ops_status` and shows:
```
Platform     Status    Capability    Last used
─────────────────────────────────────────────
Discord      ✓ live    discord       2m ago
Telegram     ✓ live    telegram      14m ago
Instagram    ✓ live    instagram     1h ago
WhatsApp     ✗ missing whatsapp      never
TikTok       ○ read    —             n/a
X            ○ manual  social:post   n/a
```

## CODIE expression
```
pug SOCIAL
├── bark platforms ← trollz_ops_status
├── if platform == discord → trollz_discord_send(msg)
├── if platform == telegram → trollz_telegram_send(msg)
├── if platform == instagram
│   ├── bark leads ← trollz_instagram_leads(handle)
│   └── biz → lead_list
├── if platform == search → trollz_web_search(query)
└── biz → display_status
```
