# Fonctionnement

## **1. Structure Globale**
### **Arborescence des modules principaux**
- **`src/`** : Cœur du plugin.
  - **`mistral/`** : Interaction avec l'API Mistral.
    - **`client.rs`** : Client HTTP pour les requêtes vers l'API Mistral (utilise `reqwest`).
    - **`controlleur/`** : Logique métier pour construire les requêtes (ex : FIM, chat).
    - **`model/`** : Modèles de données pour interagir avec l'API Mistral (ex : `Completion`, `ToolCall`).
  - **`nvim/`** : Intégration avec Neovim.
    - **`controlleur/`** : Points d'entrée pour ajouter des commandes, keymaps, et auto-commandes.
    - **`model/`** : Représentation des données pour interagir avec Neovim (ex : `Buffer`, `Cursor`).
    - **`vue/`** : Réception des messages provenant de `src/mistral/` et mise à jour de Neovim.
  - **`utils/`** : Utilitaires communs (logger, types partagés, etc).
  - **`lib.rs`** : Point d'entrée principal du plugin.
  - **`messages.rs`** : Définition des messages échangés entre `src/nvim/` et `src/mistral/`.

- **`code_modifier/`** : Bibliothèque interne pour manipuler le code (ex : parsing, injections).
- **`mistral_nvim_derive/`** : Macros procédurales pour générer des `Forms` et des `Tools`.

---

### **Rôle des modules clés**
| Module                     | Rôle                                                                                     |
|----------------------------|------------------------------------------------------------------------------------------|
| **`mistral/client`**       | Client HTTP pour interagir avec l'API Mistral (utilise `reqwest` pour les requêtes).     |
| **`mistral/model`**        | Modèles de données pour les requêtes/réponses API (ex : `Completion`, `ToolCall`).       |
| **`nvim/model`**           | Représente les données pour modifier l'état/affichage de Neovim (`Buffer`, `Cursor`).    |
| **`nvim/controlleur`**     | Points d'entrée pour initialiser le plugin (commandes, keymaps, auto-commandes).         |
| **`nvim/vue`**             | Logique d'interaction entre Mistral et Neovim (ex : mise à jour des buffers).            |
| **`nvim/model/tool_mode`** | Définition des outils mis à disposition de Mistral (ex : `CodeRefactorisation`).         |

---

## **2. Points d'entrée clés**
### **Commandes API Neovim**
- **`src/nvim/controlleur/mod.rs`** : Définit les commandes et keymaps.
  - Exemple de commande :
    ```rust
    ncmd(s, n!(FimFunction), "MistralFIMFunction", c_opts().desc("Applique FIM sur la fonction sous le curseur."))?;
    ```
  - Exemple de keymap :
    ```rust
    nmap(s, n!(FimFunction), "<Leader>mff", k_opts().desc(d).noremap(true))?;
    ```

### **Fonctions Rust principales**
- **`src/lib.rs`** : Point d'entrée du plugin.
  - Initialise les canaux de communication (`mpsc`) entre Neovim et Mistral.
  - Lance le thread `mistral_loop` pour gérer les requêtes API.

- **`src/nvim/vue/chat.rs`** : Gère les messages reçus de Mistral pour les chats.
  - Exemple de traitement d'un message :
    ```rust
    match message {
        MistralMessage::UpdateContent(chunk) => chat.lock().insert(chunk, Some(assistant_index))?,
        MistralMessage::RunTool(tool_calls) => mode.run_tool(state, run_tool_message),
    }
    ```

---

## **3. Outils**
### **Fonctionnement des outils**
- **Définition** : Les outils sont définis dans `src/mistral/model/tools.rs` et `src/nvim/model/tool_mode/`.
  - Exemple : `CodeRefactorisation` permet de modifier du code via des outils comme `CodeModifier`.
- **Exécution** :
  - Mistral envoie un `ToolCall` (ex : `CodeRetriever`).
  - Le plugin exécute l'outil et renvoie le résultat à Mistral.
  - Exemple de code :
    ```rust
    for tool in tool_calls {
        let tool_id = crate::utils::tool_id::tool_id_to_usize(tool.id.as_ref().unwrap());
        let message = mode.run_tool(state, run_tool_message);
    }
    ```

### **Créer un nouvel outil**
1. **Définir la structure** :
   - Utilise le derive `Tool` pour générer automatiquement le schéma JSON.
   - Exemple :
     ```rust
     #[derive(Tool)]
     #[description("Get a file's content.")]
     struct CodeRetriever {
         #[description("Path to the file.")]
         file: String,
     }
     ```
2. **Implémenter `Runnable`** :
   - Définir la logique dans la méthode `run`.
   - Exemple :
     ```rust
     impl Runnable for CodeRetriever {
         type Ok = String;
         type Err = String;
         fn run(&mut self, _state: SharedState, _msg: RunToolMessage) -> Result<Self::Ok, Self::Err> {
             std::fs::read_to_string(&self.file).map_err(|e| e.to_string())
         }
     }
     ```
3. **Ajouter l'outil à un mode** :
   - Dans `src/nvim/model/tool_mode/mod.rs`, ajoute l'outil à la liste des outils disponibles.
   - Exemple :
     ```rust
     impl Mode {
         pub fn current_tools(&self) -> Vec<Tool> {
             match self {
                 Self::CodeRefactorisation => CodeRefactorisation::get_tools(),
             }
         }
     }
     ```

---

## **4. Flux de données**
### **Circulation des données**
1. **Neovim → Rust** :
   - Une commande Neovim (ex : `:MistralFIMFunction`) déclenche un appel à une fonction Rust via `nvim_oxi`.
   - Exemple :
     ```rust
     ncmd(s, n!(FimFunction), "MistralFIMFunction", c_opts().desc(d))?;
     ```
2. **Rust → Tokio** :
   - La fonction Rust envoie un message via un canal `mpsc` à un thread Tokio.
   - Exemple :
     ```rust
     state.tx_mistral.send(envelop).unwrap();
     ```
3. **Tokio → API Mistral** :
   - Le thread Tokio envoie une requête HTTP à l'API Mistral via `reqwest`.
   - Exemple :
     ```rust
     self.request(reqwest::Method::POST, endpoint).body(body).send().await
     ```
4. **API Mistral → Tokio** :
   - L'API Mistral répond avec un flux (`Stream`) géré par Tokio.
   - Exemple :
     ```rust
     let mut stream = response.bytes_stream();
     while let Some(chunk) = stream.next().await { ... }
     ```
5. **Tokio → Libuv** :
   - Tokio envoie les chunks reçus à un thread Libuv via un canal `mpsc`.
   - Exemple :
     ```rust
     self.0.sendle_nvim.send_enveloppe(MistralEnveloppe { id, message });
     ```
6. **Libuv → Neovim** :
   - Le thread Libuv met à jour le buffer Neovim via `nvim_oxi`.
   - Exemple :
     ```rust
     oxi::schedule(move |_| nvim::vue::chat::handle_nvim_message(...));
     ```

---

## **5. Dépendances externes**
### **Librairies Rust utilisées**
| Librairie              | Rôle                                                                                     |
|------------------------|------------------------------------------------------------------------------------------|
| `nvim_oxi`             | Intégration avec Neovim (API nvim, gestion des buffers, etc).                            |
| `tokio`                | Runtime asynchrone pour gérer les requêtes HTTP et les flux.                             |
| `reqwest`              | Client HTTP pour interagir avec l'API Mistral.                                           |
| `serde` + `serde_json` | Sérialisation/désérialisation des données (JSON).                                        |
| `schemars`             | Génération de schémas JSON pour les outils.                                              |
| `futures`              | Gestion des streams asynchrones.                                                         |

---

## **6. Exemple concret : Workflow `:RunTests`**
### **Étapes du workflow**
1. **Déclenchement** :
   - L'utilisateur crée un chat avec `:MistralNewChat`
   - L'utilisateur rempli le formulaire de création du chat.
   - Le Chat est initialisé.
   - L'utilisateur exécute `:MistralChatSendPrompt` ou via le raccourcis `<CR><CR>`.

2. **Préparation des données** :
   - Le `controlleur` récupère le contexte (buffer, sélection, etc) et envoie un message à `src/mistral/` :
     ```rust
     let envelop = NvimEnveloppe {
         id: IdMessage::Chat(buffer.handle(), message_index),
         message: NvimMessage::Chat(request),
     };
     state.tx_mistral.send(envelop).unwrap();
     ```

3. **Envoi à Mistral** :
   - Le thread Tokio reçoit le message et envoie une requête à l'API Mistral :
     ```rust
     let response = client.request(reqwest::Method::POST, "chat/completions").body(body).send().await;
     ```

4. **Réception du flux** :
   - Mistral répond avec un flux de chunks. Chaque chunk est traité et envoyé à Neovim :
     ```rust
     while let Some(chunk) = stream.next().await {
         self.send(id, MistralMessage::UpdateContent(chunk));
     }
     ```

5. **Mise à jour de Neovim** :
   - Le thread Libuv reçoit les chunks et met à jour le buffer Neovim :
     ```rust
     match message {
         MistralMessage::UpdateContent(chunk) => chat.lock().insert(chunk, Some(assistant_index))?,
     }
     ```

6. **Exécution des outils** :
   - Si Mistral appelle un outil (ex : `CodeModifier`), le plugin l'exécute et renvoie le résultat :
     ```rust
     for tool in tool_calls {
         let message = mode.run_tool(state, run_tool_message);
         chat.push_message(message, Some(assistant_index))?;
     }
     ```

---

## **Synthèse pour intégration avec Mistral**
- **Points d'entrée** : Les commandes Neovim (ex : `:MistralFIMFunction`) et les auto-commandes (ex : sur `*.chat`) sont les portes d'entrée.
- **Outils** : Définis dans `mistral/model/tools.rs` et exécutés via `nvim/model/tool_mode/`.
- **Flux de données** :
  - Neovim → Libuv → Tokio → API Mistral → Tokio → Libuv → Neovim.
- **Extensions** :
  - Utilise `mistral_nvim_derive` pour simplifier la création d'outils et de forms.
  - Ajoute de nouveaux outils dans `src/nvim/model/tool_mode/`.
