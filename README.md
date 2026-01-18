# Mistral Neovim Plugin

Plugin permettant d'effectuer une conversation ou du FIM avec Mistral.
Le Chat conversationnel est basé sur un stockage par fichier permettant de modifier la conversation au besoin.
Ce plugin permet sélectionner une suite d'outils utilisables par Mistral, pour le moment seule la suite `CodeRefactorisation` existe.
Il est aussi possible de changer de modèle en cours de conversation, ou de modifier les réponses reçues pour corriger d'éventuelles erreurs/imprécisions.


## **Fonctionnalités**

- **Fill In the Middle (FIM)** : Complétion de code intelligente basée sur le contexte.
- **Chat interactif** : Générez du contenu avec Mistral pour refactorer, documenter ou générer du code.
- **Outils intégrés** : Utilisez des outils comme `CodeModifier`, `CodeRetriever` ou `CodeTree` pour interagir avec votre projet.
- **Personnalisation** : Créez vos propres outils et modes pour étendre les capacités du plugin.

## **Installation**

### **Prérequis**
- Neovim 0.11 ou supérieur (voir les `features` de `nvim_oxi` dans `Cargo.toml`).
- Testé avec Rust nightly 1.94 (pour la compilation).
- Une clé API Mistral (à configurer dans votre environnement).

### **Compilation manuelle**

Pour le moment, seule la compilation manuelle est supportée.

1. Clonez le dépôt :
   ```sh
   git clone https://github.com/[your-user]/mistral.nvim.git
   cd mistral.nvim
   ```

2. Compilez le plugin :
   ```sh
   cargo build --release --features=prod_mode
   plugin_path=`cwd`
   ```

3. Ajoutez un lien symbolique dans le dossier de votre configuration Neovim :
    ```sh
    cd ~/.config/nvim
    ln -s "$plugin_path/target/release/libmistral.so mistral_nvim.so"
    ```

4. Ajoutez le plugin à votre configuration Neovim :
    ```lua
    require("mistral_nvim")
    ```

## **Configuration**

### **Variables d'environnement**
- `MISTRAL_API_KEY` : Votre clé API Mistral.

### **WIP: Configuration**

Ajoutez ceci à votre `init.lua` :

```lua
require('mistral').setup {
    -- Exemple de configuration
    log_level = "info",  -- Niveaux disponibles : "trace", "debug", "info", "warn", "error", "off"
    keymaps = {
        fim_function = "<Leader>mff",  -- Raccourci pour FIM sur une fonction
        fim_visual = "<Leader>mf",    -- Raccourci pour FIM en mode visuel
    },
}
```

## **Utilisation**

### **Fill In the Middle (FIM) (FIXME : petite régression)**

1. **Sur une fonction** : Placez le curseur sur une fonction et exécutez `:MistralFIMFunction` ou utilisez le raccourci `<Leader>mff`.
2. **Sur une sélection visuelle** : Sélectionnez du code en mode visuel et exécutez `:MistralFIMVisual` ou utilisez le raccourci `<Leader>mf`.
3. **Sur la ligne du curseur** : Exécutez `:MistralFIMCursor` ou utilisez le raccourci `<Leader>mfc`.

### **Chat interactif**

1. **Créer un chat** : Ouvrez un buffer `*.chat`puis exécutez `:MistralNewChat`, remplissez le formulaire (utilisez `<tab>` pour changer de champs, `<CR>` pour valider, `<Esc>` pour annuler).
2. **Envoyer un prompt** : Écrivez votre prompt et exécutez `:MistralChatSendPrompt` ou utilisez `<CR><CR>`.
3. **Utiliser des outils** : Mistral peut appeler des outils comme `CodeRefactorisation` pour interagir avec votre code, activer des outils avec `:MistralChatChangeMode`.
4. **Changer de model** : Vous pouvez changer le modèle du prochain prompt avec `:MistralChatChangeModel`, donc une conversation peut être gérée par différents modèles.
5. **Ajuster les réponses** : Une réponse ne vous convient pas, modifiez là pour quelle colle à la réalité de votre projet.
6. **Suivez la consommation de tokens** : Une réponse ne vous convient pas, modifiez là pour quelle colle à la réalité de votre projet.
7. **Ajouter un nouveau prompt** : Pour le moment, il faut ajouter un nouveau prompt manuellement après une complétion `:MistralChatNewPrompt`.

### **Exemple de workflow**

1. Ouvrez un fichier Rust.
2. Sélectionnez une fonction en mode visuel.
3. Exécutez `:MistralFIMVisual`.
4. Mistral complète le code en fonction du contexte.

## **Financement**

Ce projet est à vocation personnelle, à moins de financement, la maintenance et l'ajout de fonctionnalités se fera en fonction de mes besoins.
Si des fonctionnalités vous intéressent, vous pouvez toujours ouvrir un ticket, comme je suis actuellement sans emploi, j'ai du temps libre.


## **Documentation technique**

Pour plus de détails sur l'architecture du projet, les contributions ou le développement de nouvelles fonctionnalités, consultez le fichier [`DEV_fr.md`](DEV_fr.md).

## **Licence**

Ce projet est sous licence **MIT**.
