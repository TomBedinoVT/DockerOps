# DockerOps Manager

Script unifié pour installer, mettre à jour, désinstaller et gérer DockerOps.

## 🚀 Installation ultra-rapide

```bash
# Installation en une ligne
curl -sSL https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh | sudo bash -s install
```

## 📋 Prérequis

- Python 3.6 ou supérieur
- curl
- Droits root (sudo) pour installation/désinstallation

## 🔧 Utilisation

### Commandes principales

```bash
# Installer la dernière version
sudo ./dockerops.sh install

# Installer une version spécifique
sudo ./dockerops.sh install -v v1.0.0

# Installer avec nettoyage complet
sudo ./dockerops.sh install --clean-all

# Désinstaller complètement
sudo ./dockerops.sh uninstall --clean-all

# Vérifier le statut
./dockerops.sh status

# Afficher l'aide
./dockerops.sh help
```

### Options d'installation

```bash
# Installer la dernière version
sudo ./dockerops.sh install

# Installer une version spécifique
sudo ./dockerops.sh install -v v1.2.0

# Installer et nettoyer la base de données
sudo ./dockerops.sh install --clean-db

# Installer et nettoyer les dossiers
sudo ./dockerops.sh install --clean-dirs

# Installer et tout nettoyer
sudo ./dockerops.sh install --clean-all
```

### Options de désinstallation

```bash
# Supprimer seulement le binaire
sudo ./dockerops.sh uninstall

# Supprimer le binaire et toutes les données
sudo ./dockerops.sh uninstall --clean-all
```

## 🔍 Fonctionnalités

### ✅ **Installation automatique**
- Détection automatique de l'architecture système
- Téléchargement automatique depuis GitHub
- Installation dans le PATH (`/usr/local/bin`)
- Sauvegarde automatique de l'ancienne version
- Restauration automatique en cas d'échec

### ✅ **Gestion des versions**
- Installation de la dernière version
- Installation de versions spécifiques
- Vérification de la version actuelle
- Mise à jour automatique

### ✅ **Nettoyage intelligent**
- Nettoyage de la base de données
- Nettoyage des dossiers de configuration
- Suppression des services systemd
- Détection des jobs cron

### ✅ **Statut et diagnostic**
- Vérification de l'installation
- Affichage de la version
- Vérification des permissions
- Informations sur la base de données
- Détection des services systemd

## 📁 Structure d'installation

```
/usr/local/bin/
└── dockerops                    # Binaire principal
    └── dockerops.backup         # Sauvegarde (si existe)

~/.dockerops/                    # Données utilisateur
├── dockerops.db                # Base de données SQLite
└── logs/                       # Logs (si configuré)
```

## 🔄 Exemples d'utilisation

### Installation propre

```bash
# Télécharger le script
curl -O https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh

# Rendre exécutable
chmod +x dockerops.sh

# Installer avec nettoyage complet
sudo ./dockerops.sh install --clean-all
```

### Mise à jour

```bash
# Mettre à jour vers la dernière version
sudo ./dockerops.sh install

# Mettre à jour vers une version spécifique
sudo ./dockerops.sh install -v v1.3.0
```

### Désinstallation

```bash
# Désinstaller complètement
sudo ./dockerops.sh uninstall --clean-all
```

### Diagnostic

```bash
# Vérifier l'installation
./dockerops.sh status
```

## 🛠️ Résolution de problèmes

### Erreur de permissions

```bash
# Vérifier les permissions
ls -la /usr/local/bin/dockerops

# Corriger les permissions si nécessaire
sudo chmod +x /usr/local/bin/dockerops
```

### Erreur de téléchargement

```bash
# Vérifier la connectivité
curl -I https://api.github.com

# Utiliser un proxy si nécessaire
export https_proxy=http://proxy:port
```

### Problème de version

```bash
# Vérifier la version installée
dockerops version

# Réinstaller si nécessaire
sudo ./dockerops.sh install --clean-all
```

### Erreur Python

```bash
# Vérifier Python 3
python3 --version

# Installer Python 3 si nécessaire
sudo apt update && sudo apt install python3
```

## 🔒 Sécurité

- ✅ **Vérification des permissions** root
- ✅ **Sauvegarde automatique** avant modification
- ✅ **Nettoyage des fichiers temporaires**
- ✅ **Gestion des erreurs** avec restauration
- ✅ **Validation de l'installation**

## 📝 Scripts de maintenance

### Mise à jour automatique

```bash
#!/bin/bash
# Script de maintenance automatique

echo "🔄 Mise à jour de DockerOps..."
sudo ./dockerops.sh install --clean-db
echo "✅ Mise à jour terminée"
```

### Vérification quotidienne

```bash
#!/bin/bash
# Script de vérification quotidienne

echo "🔍 Vérification de DockerOps..."
./dockerops.sh status
```

## 🔄 Mise à jour du script

Pour mettre à jour le script manager :

```bash
# Télécharger la dernière version
curl -O https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh

# Rendre exécutable
chmod +x dockerops.sh
```

## 📞 Support

En cas de problème :

1. Vérifiez les logs d'erreur
2. Testez avec `--clean-all`
3. Vérifiez la connectivité Internet
4. Consultez la documentation DockerOps

## 🎯 Avantages du script unifié

1. **Un seul script** pour toutes les opérations
2. **Interface cohérente** avec sous-commandes
3. **Gestion automatique** des dépendances
4. **Nettoyage automatique** des fichiers temporaires
5. **Documentation intégrée** avec `help`
6. **Installation en une ligne** possible 