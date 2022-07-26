# source this file to setup env vars and docker db container for local development
# of the server

export POSTGRES_HOST="localhost"
export POSTGRES_USER="developer"
export POSTGRES_DB="ddj"
export POSTGRES_PASSWORD="password"
alias restart-db="sudo docker-compose -f db-only.yml down -v; sudo docker-compose -f db-only.yml up -d"
alias db-done="sudo docker-compose -f db-only.yml down -v; unset POSTGRES_USER; unset POSTGRES_DB; unset POSTGRES_PASSWORD; unset POSTGRES_HOST"
sudo docker-compose -f db-only.yml up -d
echo "dev env configured - run 'restart-db' for a fresh db - run 'db-done' to destroy db"
