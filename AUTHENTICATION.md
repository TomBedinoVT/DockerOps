# Authentification GitHub pour DockerOps

## Problème

Si vous rencontrez l'erreur suivante lors de l'utilisation de DockerOps :

```
Error: Failed to clone repository: remote authentication required but no callback set; class=Http (34); code=Auth (-16)
```

Cela signifie que le repository GitHub nécessite une authentification.

## Solutions

### Solution 1: Token GitHub (Recommandée)

1. **Créer un token GitHub :**
   - Allez sur GitHub.com → Settings → Developer settings → Personal access tokens → Tokens (classic)
   - Cliquez sur "Generate new token (classic)"
   - Donnez un nom à votre token (ex: "DockerOps")
   - Sélectionnez les permissions nécessaires :
     - `repo` (pour les repositories privés)
     - `read:org` (si vous accédez à des repositories d'organisation)
   - Cliquez sur "Generate token"
   - **Copiez le token** (vous ne pourrez plus le voir après)

2. **Configurer le token :**

   **Windows (PowerShell) :**
   ```powershell
   $env:GITHUB_TOKEN="ghp_votre_token_ici"
   ```

   **Windows (Command Prompt) :**
   ```cmd
   set GITHUB_TOKEN=ghp_votre_token_ici
   ```

   **Linux/macOS :**
   ```bash
   export GITHUB_TOKEN="ghp_votre_token_ici"
   ```

3. **Utiliser DockerOps :**
   ```bash
   dockerops watch https://github.com/username/repository
   ```

### Solution 2: Configuration permanente

Pour éviter de redéfinir la variable d'environnement à chaque fois :

**Windows :**
1. Ouvrez les Paramètres système → Variables d'environnement
2. Ajoutez une nouvelle variable utilisateur :
   - Nom : `GITHUB_TOKEN`
   - Valeur : `ghp_votre_token_ici`

**Linux/macOS :**
Ajoutez à votre fichier `~/.bashrc` ou `~/.zshrc` :
```bash
export GITHUB_TOKEN="ghp_votre_token_ici"
```

### Solution 3: Repository public

Si le repository est public, vous pouvez essayer de le rendre public sur GitHub pour éviter l'authentification.

## Sécurité

- **Ne commitez jamais votre token** dans votre code
- **Utilisez des tokens avec des permissions minimales**
- **Régénérez régulièrement vos tokens**
- **Supprimez les tokens inutilisés**

## Dépannage

Si vous avez toujours des problèmes :

1. Vérifiez que le token est correctement défini :
   ```bash
   echo $GITHUB_TOKEN  # Linux/macOS
   echo %GITHUB_TOKEN% # Windows CMD
   $env:GITHUB_TOKEN   # Windows PowerShell
   ```

2. Vérifiez que le token a les bonnes permissions

3. Vérifiez que l'URL du repository est correcte

4. Essayez de cloner manuellement pour tester :
   ```bash
   git clone https://github.com/username/repository.git
   ``` 