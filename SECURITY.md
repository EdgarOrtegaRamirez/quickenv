# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | ✅ Active development |

## Encryption

QuickEnv uses AES-256-GCM for .env file encryption with Argon2id key derivation:
- **Key derivation**: Argon2id (memory-hard, resistant to GPU/ASIC attacks)
- **Encryption**: AES-256-GCM (authenticated encryption with associated data)
- **Salt**: 16 random bytes, unique per encryption
- **Nonce**: 12 random bytes, unique per encryption
- **Output format**: salt (16) + nonce (12) + ciphertext

## Security Best Practices

1. **Never commit .env files** containing real secrets to version control
2. Use `.env.example` with dummy values as a template
3. Encrypt sensitive .env files before storage or transit
4. Use strong, unique passphrases for encryption
5. Add `.env.encrypted` and `.env.decrypted` to your `.gitignore`

## Reporting a Vulnerability

If you discover a security vulnerability, please open an issue or contact the repository owner directly. Do not disclose security vulnerabilities publicly until they have been addressed.