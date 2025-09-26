# 📌 IoT Project – API Simples em Rust com Axum + PostgreSQL

Este projeto é uma API básica feita em **Rust** usando **Axum** e **SQLx**.  
Ela permite:

- ✅ Inserir dados no banco via `POST /items`
- ✅ Listar dados em JSON via `GET /items`
- ✅ Visualizar os dados em uma página HTML via `GET /`

---

## 📂 Estrutura do Projeto

```
iot_project/
 ├─ Cargo.toml        # Dependências do projeto
 ├─ .env              # Configurações de ambiente (não versionar!)
 └─ src/
     ├─ main.rs       # Entrada da aplicação
     ├─ db.rs         # Conexão e schema do banco
     └─ models.rs     # Estruturas de dados (DTOs)
```

---

## ⚙️ Dependências principais

- [Axum](https://github.com/tokio-rs/axum) – framework web em Rust
- [Tokio](https://tokio.rs/) – runtime assíncrono
- [SQLx](https://github.com/launchbadge/sqlx) – ORM para Rust
- [dotenvy](https://github.com/allan2/dotenvy) – leitura de variáveis do `.env`
- [UUID](https://crates.io/crates/uuid) – geração de IDs únicos

---

## 🔑 Configuração do Banco de Dados

### 1. Criar banco no PostgreSQL
Entre no `psql` e rode:
```sql
CREATE DATABASE iot_project;
CREATE USER iot_user WITH PASSWORD 'SenhaForteAqui!';
GRANT ALL PRIVILEGES ON DATABASE iot_project TO iot_user;
```

### 2. Configurar `.env`
Crie o arquivo `.env` na raiz do projeto:

```env
DATABASE_URL=postgres://iot_user:SenhaForteAqui!@localhost:5432/iot_project
```

⚠️ Se sua senha tiver caracteres especiais (`! ? @ $ " ...`), use **percent-encoding**.  
Exemplo: `Senha!123` → `Senha%21123`.

---

## ▶️ Rodando o Projeto

### 1. Clonar e entrar na pasta
```bash
git clone ...
cd iot_project
```

### 2. Compilar
```bash
cargo build
```

### 3. Rodar
```bash
cargo run
```

### 4. Saída esperada
```
✅ Conectado ao banco PostgreSQL em: postgres://iot_user@localhost:5432/iot_project
🚀 Servidor rodando em http://0.0.0.0:42351
```

> A porta é aleatória (escolhida pelo SO). Veja no log qual porta foi aberta.

---

## 🔍 Testando a API

### Inserir um item
```bash
curl -X POST http://localhost:42351/items   -H "Content-Type: application/json"   -d '{"nome":"Sensor de Temperatura"}'
```

### Listar itens em JSON
```bash
curl http://localhost:42351/items
```

### Ver página HTML
Abra no navegador:
```
http://localhost:42351
```

---

## 🛠️ Problemas comuns

- **`pool timed out while waiting for an open connection`**  
  → Banco não está rodando ou senha incorreta no `.env`.  
  → Teste conexão manual com:
  ```bash
  psql $DATABASE_URL
  ```

- **Senha com caracteres especiais não funciona**  
  → Use **percent-encoding** no `.env`.

- **Porta muda a cada execução**  
  → O servidor usa porta aleatória (`0.0.0.0:0`). Veja no log a porta atual.

---

## 📌 Próximos Passos

- Adicionar autenticação JWT
- Criar endpoints de atualização (`PUT /items/:id`) e exclusão (`DELETE /items/:id`)
- Subir em um container com Docker Compose (Postgres + API)  