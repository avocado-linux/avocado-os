echo "Compiling Elixir application"
cd ref-elixir
mix deps.get
mix compile 
mix release --overwrite
