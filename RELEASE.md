# Release Process

Ce document explique comment publier une nouvelle version de DockerOps CLI.

## Workflow GitHub Actions

Le projet utilise un workflow GitHub Actions qui se lance manuellement pour créer des releases.

### Déclenchement manuel

1. Allez sur l'onglet **Actions** du repository GitHub
2. Sélectionnez le workflow **Release**
3. Cliquez sur **Run workflow**
4. Remplissez les champs :
   - **Version** : Version à publier (ex: `v1.0.0`)
   - **Release notes** : Notes de version (optionnel)

### Ce que fait le workflow

1. **Compilation Linux** :
   - Linux (x86_64)

2. **Optimisation** :
   - Compilation en mode release
   - Stripping des symboles de debug

3. **Création de la release** :
   - Tag Git automatique
   - Release GitHub avec notes
   - Upload des binaires
   - Génération des checksums SHA256

### Assets générés

Pour chaque release, les fichiers suivants sont créés :

- `dockerops-linux-x86_64` - Binaire Linux
- `dockerops-linux-x86_64.sha256` - Fichier de checksum pour vérification

## Installation des releases

### Linux
```bash
# Télécharger
wget https://github.com/user/repo/releases/download/v1.0.0/dockerops-linux-x86_64

# Vérifier le checksum
echo "checksum_here" | sha256sum -c dockerops-linux-x86_64.sha256

# Installer
chmod +x dockerops-linux-x86_64
sudo mv dockerops-linux-x86_64 /usr/local/bin/dockerops
```

## Convention de versioning

Utilisez le [Semantic Versioning](https://semver.org/) :

- **MAJOR** : Changements incompatibles avec les versions précédentes
- **MINOR** : Nouvelles fonctionnalités compatibles
- **PATCH** : Corrections de bugs compatibles

Exemples :
- `v1.0.0` - Première version stable
- `v1.1.0` - Nouvelles fonctionnalités
- `v1.1.1` - Corrections de bugs
- `v2.0.0` - Version majeure avec changements incompatibles 