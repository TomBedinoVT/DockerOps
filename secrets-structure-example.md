# Structure des Secrets DockerOps

## Organisation des fichiers

```
mon-stack/
├── docker-compose.yml
├── secrets.yaml          # Définition des secrets pour ce stack
└── ...

/secrets/                 # Répertoire racine des secrets (doit exister sur le système)
├── database_password     # Contenu: "mon_mot_de_passe_secret"
├── api_key              # Contenu: "ma_cle_api_secrete"
├── jwt_secret           # Contenu: "mon_secret_jwt"
└── redis_password       # Contenu: "mot_de_passe_redis"
```

## Format du fichier secrets.yaml

Le fichier `secrets.yaml` doit être placé dans le dossier de chaque stack et contient la liste des secrets à injecter :

```yaml
- id: database_password
  env: DB_PASSWORD

- id: api_key
  env: API_SECRET_KEY
```

## Fonctionnement

1. **Lecture du fichier secrets.yaml** : DockerOps lit le fichier `secrets.yaml` dans le dossier du stack
2. **Vérification des secrets** : Pour chaque secret défini, DockerOps vérifie que le fichier `/secrets/{id}` existe
3. **Injection comme variables d'environnement** : Les secrets sont passés comme variables d'environnement au processus `docker stack deploy`
4. **Erreur si secret manquant** : Si un fichier secret n'existe pas, DockerOps génère une erreur et arrête le déploiement

## Exemple de transformation

### Avant (docker-compose.yml original)
```yaml
services:
  app:
    image: mon-app:latest
    environment:
      - NODE_ENV=production
```

### Après (avec secrets injectés)
Le fichier docker-compose reste inchangé, mais les secrets sont disponibles comme variables d'environnement pour le processus `docker stack deploy` :

```bash
# DockerOps exécute :
DB_PASSWORD=mon_mot_de_passe_secret \
API_SECRET_KEY=ma_cle_api_secrete \
docker stack deploy --detach=false -c docker-compose.yml mon-stack
```

## Sécurité

- Les secrets sont lus depuis `/secrets/` qui doit être protégé avec les bonnes permissions
- Les valeurs des secrets ne sont jamais loggées (seuls les noms des variables d'environnement sont affichés)
- Les secrets ne sont jamais écrits dans les fichiers docker-compose
- Les secrets sont passés uniquement comme variables d'environnement au processus `docker stack deploy`
- Si un secret est manquant, le déploiement échoue pour éviter les problèmes de sécurité 