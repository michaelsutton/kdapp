# 🏆 Session 4: Kaspa Poker Tournament (Architecture Validation)

**Prerequisites**: Session 3 completed (NPM SDKs ready)

## 🎯 **Session Goal: Prove the Architecture Scales**

**Why This**: Build a complex multi-participant application using the same kdapp authentication architecture. This validates that your P2P approach works for real applications beyond simple auth.

**Time Estimate**: 5-6 hours  
**Outcome**: Working poker tournament that demonstrates kaspa-auth enables sophisticated P2P applications

---

## 📋 **Phase 1: Design Poker Episode (60 minutes)**

### 1.1 Define Poker Game Rules (20 min)
```rust
// examples/kaspa-poker/src/core/game_state.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokerGameState {
    pub game_id: u64,
    pub players: Vec<Player>,
    pub current_hand: Option<Hand>,
    pub pot: u64,            // In sompi
    pub buy_in: u64,         // Required buy-in amount
    pub max_players: u8,     // Usually 6-9
    pub status: GameStatus,
    pub dealer_position: usize,
    pub community_cards: Vec<Card>,
    pub betting_round: BettingRound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub public_key: String,
    pub session_token: String,  // From kaspa-auth!
    pub chip_count: u64,
    pub position: u8,
    pub hole_cards: Option<[Card; 2]>,
    pub current_bet: u64,
    pub status: PlayerStatus,
    pub has_acted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameStatus {
    WaitingForPlayers,
    InProgress,
    HandComplete,
    GameComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BettingRound {
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
}
```

### 1.2 Define Poker Commands (25 min)
```rust
// examples/kaspa-poker/src/core/commands.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PokerCommand {
    // Game management
    CreateGame {
        buy_in: u64,
        max_players: u8,
    },
    JoinGame {
        game_id: u64,
        session_token: String,  // Must be authenticated via kaspa-auth
        buy_in_utxo: String,    // UTXO for buy-in payment
    },
    
    // Game actions
    Fold { game_id: u64 },
    Call { game_id: u64 },
    Raise { 
        game_id: u64, 
        amount: u64 
    },
    AllIn { game_id: u64 },
    
    // Card management (commitment-reveal)
    CommitDeck { 
        game_id: u64, 
        deck_commitment: String  // Hash of shuffled deck
    },
    RevealCards { 
        game_id: u64, 
        cards: Vec<Card>,
        nonce: String 
    },
}
```

### 1.3 Design Authentication Integration (15 min)
```rust
// examples/kaspa-poker/src/core/auth_integration.rs
pub struct AuthenticatedPokerEpisode {
    pub poker_state: PokerGameState,
    pub auth_verifier: SessionVerifier,  // From kaspa-auth
}

impl Episode for AuthenticatedPokerEpisode {
    fn execute(&mut self, command: &PokerCommand, auth: Option<PubKey>, meta: &PayloadMetadata) -> Result<Rollback, Error> {
        match command {
            PokerCommand::JoinGame { session_token, .. } => {
                // Verify session token using kaspa-auth
                if !self.auth_verifier.verify_session(session_token, auth)? {
                    return Err("Invalid session token".into());
                }
                
                // Add player to game
                self.add_player_to_game(auth.unwrap(), session_token)?;
                Ok(Rollback::PlayerJoined { /* rollback data */ })
            }
            
            PokerCommand::Fold { game_id } => {
                // Verify player is authenticated and in game
                let player = self.get_authenticated_player(auth)?;
                self.fold_player(player)?;
                Ok(Rollback::PlayerFolded { /* rollback data */ })
            }
            
            // ... other poker actions
        }
    }
}
```

---

## 📋 **Phase 2: Implement Core Poker Logic (120 minutes)**

### 2.1 Game State Management (45 min)
```rust
// examples/kaspa-poker/src/core/episode.rs
impl AuthenticatedPokerEpisode {
    pub fn new() -> Self {
        Self {
            poker_state: PokerGameState::new(),
            auth_verifier: SessionVerifier::new(),
        }
    }
    
    fn add_player_to_game(&mut self, public_key: PubKey, session_token: &str) -> Result<(), Error> {
        if self.poker_state.players.len() >= self.poker_state.max_players as usize {
            return Err("Game is full".into());
        }
        
        let player = Player {
            public_key: public_key.to_string(),
            session_token: session_token.to_string(),
            chip_count: self.poker_state.buy_in,
            position: self.poker_state.players.len() as u8,
            hole_cards: None,
            current_bet: 0,
            status: PlayerStatus::Active,
            has_acted: false,
        };
        
        self.poker_state.players.push(player);
        
        // Start game if enough players
        if self.poker_state.players.len() >= 2 {
            self.start_new_hand()?;
        }
        
        Ok(())
    }
    
    fn start_new_hand(&mut self) -> Result<(), Error> {
        // Deal hole cards (simplified - in production use commitment-reveal)
        for player in &mut self.poker_state.players {
            if player.status == PlayerStatus::Active {
                player.hole_cards = Some(self.deal_hole_cards());
                player.current_bet = 0;
                player.has_acted = false;
            }
        }
        
        self.poker_state.betting_round = BettingRound::PreFlop;
        self.poker_state.community_cards.clear();
        
        Ok(())
    }
    
    fn fold_player(&mut self, player_key: &PubKey) -> Result<(), Error> {
        if let Some(player) = self.find_player_mut(player_key) {
            player.status = PlayerStatus::Folded;
            player.has_acted = true;
            
            // Check if hand is over
            let active_players = self.count_active_players();
            if active_players <= 1 {
                self.end_hand()?;
            }
            
            Ok(())
        } else {
            Err("Player not found".into())
        }
    }
    
    fn process_bet(&mut self, player_key: &PubKey, amount: u64) -> Result<(), Error> {
        if let Some(player) = self.find_player_mut(player_key) {
            if player.chip_count < amount {
                return Err("Insufficient chips".into());
            }
            
            player.chip_count -= amount;
            player.current_bet += amount;
            player.has_acted = true;
            
            // Add to pot
            self.poker_state.pot += amount;
            
            // Check if betting round is complete
            if self.all_players_acted() {
                self.advance_betting_round()?;
            }
            
            Ok(())
        } else {
            Err("Player not found".into())
        }
    }
}
```

### 2.2 Betting Logic (45 min)
```rust
impl AuthenticatedPokerEpisode {
    fn advance_betting_round(&mut self) -> Result<(), Error> {
        match self.poker_state.betting_round {
            BettingRound::PreFlop => {
                self.deal_flop();
                self.poker_state.betting_round = BettingRound::Flop;
            }
            BettingRound::Flop => {
                self.deal_turn();
                self.poker_state.betting_round = BettingRound::Turn;
            }
            BettingRound::Turn => {
                self.deal_river();
                self.poker_state.betting_round = BettingRound::River;
            }
            BettingRound::River => {
                self.poker_state.betting_round = BettingRound::Showdown;
                self.determine_winner()?;
            }
            BettingRound::Showdown => {
                self.end_hand()?;
            }
        }
        
        // Reset betting for new round
        for player in &mut self.poker_state.players {
            player.current_bet = 0;
            player.has_acted = false;
        }
        
        Ok(())
    }
    
    fn determine_winner(&mut self) -> Result<(), Error> {
        let active_players: Vec<&Player> = self.poker_state.players
            .iter()
            .filter(|p| p.status == PlayerStatus::Active)
            .collect();
            
        if active_players.len() == 1 {
            // Only one player left
            let winner = active_players[0];
            self.award_pot_to_player(&winner.public_key)?;
        } else {
            // Showdown - evaluate hands
            let winner = self.evaluate_hands(&active_players)?;
            self.award_pot_to_player(&winner.public_key)?;
        }
        
        Ok(())
    }
    
    fn evaluate_hands(&self, players: &[&Player]) -> Result<&Player, Error> {
        // Simplified hand evaluation
        // In production: implement proper poker hand rankings
        let mut best_player = players[0];
        let mut best_hand_rank = self.evaluate_hand(best_player)?;
        
        for &player in players.iter().skip(1) {
            let hand_rank = self.evaluate_hand(player)?;
            if hand_rank > best_hand_rank {
                best_player = player;
                best_hand_rank = hand_rank;
            }
        }
        
        Ok(best_player)
    }
}
```

### 2.3 Card Management (30 min)
```rust
// examples/kaspa-poker/src/core/cards.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Suit {
    Hearts, Diamonds, Clubs, Spades
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Rank {
    Two = 2, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace
}

impl AuthenticatedPokerEpisode {
    fn deal_hole_cards(&self) -> [Card; 2] {
        // Simplified - in production use proper deck shuffling and commitment-reveal
        [
            Card { suit: Suit::Hearts, rank: Rank::Ace },
            Card { suit: Suit::Spades, rank: Rank::King },
        ]
    }
    
    fn deal_flop(&mut self) {
        // Deal 3 community cards
        self.poker_state.community_cards.extend([
            Card { suit: Suit::Diamonds, rank: Rank::Queen },
            Card { suit: Suit::Clubs, rank: Rank::Jack },
            Card { suit: Suit::Hearts, rank: Rank::Ten },
        ]);
    }
    
    fn deal_turn(&mut self) {
        // Deal 4th community card
        self.poker_state.community_cards.push(
            Card { suit: Suit::Spades, rank: Rank::Nine }
        );
    }
    
    fn deal_river(&mut self) {
        // Deal 5th community card
        self.poker_state.community_cards.push(
            Card { suit: Suit::Hearts, rank: Rank::Eight }
        );
    }
    
    fn evaluate_hand(&self, player: &Player) -> Result<u32, Error> {
        // Simplified hand evaluation
        // In production: implement proper poker hand rankings (flush, straight, etc.)
        if let Some(hole_cards) = player.hole_cards {
            let mut all_cards = hole_cards.to_vec();
            all_cards.extend(&self.poker_state.community_cards);
            
            // Return simplified hand rank (high card value)
            Ok(all_cards.iter().map(|c| c.rank as u32).max().unwrap_or(0))
        } else {
            Err("Player has no hole cards".into())
        }
    }
}
```

---

## 📋 **Phase 3: Authentication Integration (75 minutes)**

### 3.1 Session Verification (30 min)
```rust
// examples/kaspa-poker/src/auth/session_verifier.rs
use kaspa_auth_core::{AuthCommand, AuthEpisode};

pub struct SessionVerifier {
    pub active_sessions: HashMap<String, AuthSession>,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub public_key: String,
    pub session_token: String,
    pub expires_at: SystemTime,
    pub episode_id: u64,
}

impl SessionVerifier {
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
        }
    }
    
    pub fn verify_session(&self, session_token: &str, public_key: Option<PubKey>) -> Result<bool, Error> {
        if let Some(session) = self.active_sessions.get(session_token) {
            // Check expiry
            if SystemTime::now() > session.expires_at {
                return Ok(false);
            }
            
            // Check public key matches
            if let Some(pk) = public_key {
                if session.public_key != pk.to_string() {
                    return Ok(false);
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn add_session(&mut self, session: AuthSession) {
        self.active_sessions.insert(session.session_token.clone(), session);
    }
    
    pub fn remove_session(&mut self, session_token: &str) {
        self.active_sessions.remove(session_token);
    }
}
```

### 3.2 Poker-Auth Bridge (25 min)
```rust
// examples/kaspa-poker/src/auth/bridge.rs
use kaspa_auth_core::AuthEpisode;

pub struct PokerAuthBridge {
    pub auth_episode: AuthEpisode,
    pub session_verifier: SessionVerifier,
}

impl PokerAuthBridge {
    pub fn new() -> Self {
        Self {
            auth_episode: AuthEpisode::new(),
            session_verifier: SessionVerifier::new(),
        }
    }
    
    pub fn handle_auth_event(&mut self, event: AuthEvent) -> Result<(), Error> {
        match event {
            AuthEvent::AuthenticationComplete { public_key, session_token, episode_id } => {
                let session = AuthSession {
                    public_key: public_key.to_string(),
                    session_token: session_token.clone(),
                    expires_at: SystemTime::now() + Duration::from_secs(3600),
                    episode_id,
                };
                
                self.session_verifier.add_session(session);
                println!("✅ Player authenticated and ready for poker: {}", public_key);
                Ok(())
            }
            
            AuthEvent::SessionRevoked { session_token } => {
                self.session_verifier.remove_session(&session_token);
                println!("🚪 Player session revoked: {}", session_token);
                Ok(())
            }
        }
    }
    
    pub fn verify_player_authenticated(&self, session_token: &str, public_key: &PubKey) -> bool {
        self.session_verifier.verify_session(session_token, Some(public_key.clone())).unwrap_or(false)
    }
}
```

### 3.3 Integrate with Poker Episode (20 min)
```rust
// Update examples/kaspa-poker/src/core/episode.rs
impl Episode for AuthenticatedPokerEpisode {
    fn execute(&mut self, command: &PokerCommand, auth: Option<PubKey>, meta: &PayloadMetadata) -> Result<Rollback, Error> {
        match command {
            PokerCommand::JoinGame { session_token, game_id, buy_in_utxo } => {
                // 1. Verify authentication
                let public_key = auth.ok_or("Authentication required")?;
                if !self.auth_bridge.verify_player_authenticated(session_token, &public_key) {
                    return Err("Invalid or expired session token".into());
                }
                
                // 2. Verify buy-in payment (simplified)
                if !self.verify_buy_in_payment(buy_in_utxo, &public_key)? {
                    return Err("Invalid buy-in payment".into());
                }
                
                // 3. Add to game
                self.add_player_to_game(public_key, session_token)?;
                
                Ok(Rollback::PlayerJoined { 
                    player_key: public_key.to_string(),
                    session_token: session_token.clone()
                })
            }
            
            PokerCommand::Fold { game_id } => {
                let public_key = auth.ok_or("Authentication required")?;
                
                // Verify player is in game and authenticated
                if !self.is_player_in_game(&public_key)? {
                    return Err("Player not in game".into());
                }
                
                self.fold_player(&public_key)?;
                Ok(Rollback::PlayerFolded { player_key: public_key.to_string() })
            }
            
            PokerCommand::Raise { game_id, amount } => {
                let public_key = auth.ok_or("Authentication required")?;
                self.process_bet(&public_key, *amount)?;
                Ok(Rollback::PlayerBet { 
                    player_key: public_key.to_string(), 
                    amount: *amount 
                })
            }
            
            // ... other poker commands
        }
    }
}
```

---

## 📋 **Phase 4: WebSocket Poker Interface (90 minutes)**

### 4.1 Create Poker WebSocket Server (45 min)
```rust
// examples/kaspa-poker/src/api/websocket.rs
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct PokerWebSocketServer {
    pub auth_bridge: Arc<Mutex<PokerAuthBridge>>,
    pub poker_episode: Arc<Mutex<AuthenticatedPokerEpisode>>,
    pub broadcast_tx: broadcast::Sender<PokerEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PokerEvent {
    GameCreated { game_id: u64, buy_in: u64 },
    PlayerJoined { game_id: u64, player: String, players_count: u8 },
    HandStarted { game_id: u64, dealer_position: u8 },
    CardsDealt { game_id: u64, community_cards: Vec<Card> },
    PlayerAction { game_id: u64, player: String, action: String, amount: Option<u64> },
    HandComplete { game_id: u64, winner: String, pot: u64 },
    GameComplete { game_id: u64, final_standings: Vec<String> },
}

impl PokerWebSocketServer {
    pub async fn handle_connection(&self, stream: TcpStream) {
        let ws_stream = accept_async(stream).await.expect("WebSocket handshake");
        let (ws_tx, mut ws_rx) = ws_stream.split();
        
        let mut broadcast_rx = self.broadcast_tx.subscribe();
        
        // Handle incoming messages from client
        let broadcast_tx = self.broadcast_tx.clone();
        let poker_episode = self.poker_episode.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<PokerClientMessage>(&text) {
                        Self::handle_client_message(client_msg, &poker_episode, &broadcast_tx).await;
                    }
                }
            }
        });
        
        // Broadcast poker events to client
        tokio::spawn(async move {
            while let Ok(event) = broadcast_rx.recv().await {
                let event_json = serde_json::to_string(&event).unwrap();
                if ws_tx.send(Message::Text(event_json)).await.is_err() {
                    break;
                }
            }
        });
    }
    
    async fn handle_client_message(
        msg: PokerClientMessage, 
        poker_episode: &Arc<Mutex<AuthenticatedPokerEpisode>>,
        broadcast_tx: &broadcast::Sender<PokerEvent>
    ) {
        match msg {
            PokerClientMessage::JoinGame { session_token, game_id, public_key } => {
                let mut episode = poker_episode.lock().await;
                
                // Submit JoinGame command to kdapp episode
                let join_cmd = PokerCommand::JoinGame {
                    game_id,
                    session_token: session_token.clone(),
                    buy_in_utxo: "placeholder_utxo".to_string(),
                };
                
                match episode.execute(&join_cmd, Some(public_key), &PayloadMetadata::default()) {
                    Ok(_) => {
                        let _ = broadcast_tx.send(PokerEvent::PlayerJoined {
                            game_id,
                            player: public_key.to_string(),
                            players_count: episode.poker_state.players.len() as u8,
                        });
                    }
                    Err(e) => {
                        println!("Failed to join game: {}", e);
                    }
                }
            }
            
            PokerClientMessage::PlayerAction { game_id, action, amount } => {
                let mut episode = poker_episode.lock().await;
                
                let poker_cmd = match action.as_str() {
                    "fold" => PokerCommand::Fold { game_id },
                    "call" => PokerCommand::Call { game_id },
                    "raise" => PokerCommand::Raise { game_id, amount: amount.unwrap_or(0) },
                    _ => return,
                };
                
                // Execute poker action
                if let Ok(_) = episode.execute(&poker_cmd, Some(msg.public_key), &PayloadMetadata::default()) {
                    let _ = broadcast_tx.send(PokerEvent::PlayerAction {
                        game_id,
                        player: msg.public_key.to_string(),
                        action,
                        amount,
                    });
                }
            }
        }
    }
}
```

### 4.2 Create Poker Frontend (45 min)
```html
<!-- examples/kaspa-poker/public/poker.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Kaspa Poker Tournament</title>
    <style>
        .poker-table {
            background: #0f5132;
            border-radius: 50%;
            width: 600px;
            height: 400px;
            margin: 20px auto;
            position: relative;
            border: 3px solid #8b5a3c;
        }
        
        .player-seat {
            position: absolute;
            width: 100px;
            height: 80px;
            background: #fff;
            border-radius: 8px;
            text-align: center;
            padding: 10px;
            border: 2px solid #333;
        }
        
        .community-cards {
            position: absolute;
            top: 40%;
            left: 50%;
            transform: translate(-50%, -50%);
            display: flex;
            gap: 10px;
        }
        
        .card {
            width: 50px;
            height: 70px;
            background: white;
            border: 1px solid #333;
            border-radius: 5px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: bold;
        }
        
        .actions {
            margin: 20px;
            text-align: center;
        }
        
        .actions button {
            margin: 5px;
            padding: 10px 20px;
            font-size: 16px;
            border: none;
            border-radius: 5px;
            cursor: pointer;
        }
        
        .fold { background: #dc3545; color: white; }
        .call { background: #28a745; color: white; }
        .raise { background: #ffc107; color: black; }
    </style>
</head>
<body>
    <div id="app">
        <h1>🃏 Kaspa Poker Tournament</h1>
        
        <!-- Authentication Section -->
        <div id="auth-section">
            <h3>🔐 Authentication Required</h3>
            <p>You must authenticate with Kaspa before joining the poker game.</p>
            <button id="auth-btn">🚀 Authenticate with Kaspa</button>
            <div id="auth-status"></div>
        </div>
        
        <!-- Poker Game Section -->
        <div id="poker-section" style="display: none;">
            <div id="game-info">
                <p>Game ID: <span id="game-id">12345</span></p>
                <p>Pot: <span id="pot-amount">0</span> KAS</p>
                <p>Your Chips: <span id="chip-count">1000</span> KAS</p>
            </div>
            
            <div class="poker-table">
                <!-- Player seats will be populated dynamically -->
                <div id="player-seats"></div>
                
                <!-- Community cards -->
                <div class="community-cards" id="community-cards">
                    <!-- Cards will be added dynamically -->
                </div>
            </div>
            
            <!-- Player actions -->
            <div class="actions" id="player-actions" style="display: none;">
                <button class="fold" onclick="poker.fold()">🚫 Fold</button>
                <button class="call" onclick="poker.call()">✅ Call</button>
                <button class="raise" onclick="poker.showRaiseDialog()">📈 Raise</button>
                
                <div id="raise-dialog" style="display: none;">
                    <input type="number" id="raise-amount" placeholder="Raise amount" min="1">
                    <button onclick="poker.raise()">Confirm Raise</button>
                </div>
            </div>
        </div>
        
        <!-- Game log -->
        <div id="game-log">
            <h4>🎯 Game Events</h4>
            <div id="log-messages"></div>
        </div>
    </div>
    
    <script>
        class KaspaPoker {
            constructor() {
                this.ws = null;
                this.gameId = 12345;
                this.sessionToken = null;
                this.publicKey = null;
                this.isAuthenticated = false;
                
                this.setupEventListeners();
            }
            
            setupEventListeners() {
                document.getElementById('auth-btn').onclick = () => this.authenticate();
            }
            
            async authenticate() {
                // First authenticate with kaspa-auth
                try {
                    document.getElementById('auth-status').textContent = '🔄 Authenticating...';
                    
                    // Open kaspa-auth in popup
                    const authWindow = window.open(
                        'http://localhost:8080', 
                        'kaspa-auth', 
                        'width=500,height=600'
                    );
                    
                    // Listen for authentication success
                    window.addEventListener('message', (event) => {
                        if (event.data.type === 'kaspa-auth-success') {
                            this.sessionToken = event.data.sessionToken;
                            this.publicKey = event.data.publicKey;
                            this.isAuthenticated = true;
                            
                            authWindow.close();
                            this.onAuthenticationComplete();
                        }
                    });
                    
                } catch (error) {
                    document.getElementById('auth-status').textContent = '❌ Authentication failed';
                    console.error('Authentication error:', error);
                }
            }
            
            onAuthenticationComplete() {
                document.getElementById('auth-section').style.display = 'none';
                document.getElementById('poker-section').style.display = 'block';
                
                this.logMessage('✅ Authentication successful! Connecting to poker game...');
                this.connectToPoker();
            }
            
            connectToPoker() {
                this.ws = new WebSocket('ws://localhost:8081'); // Different port for poker
                
                this.ws.onopen = () => {
                    this.logMessage('🟢 Connected to poker server');
                    this.joinGame();
                };
                
                this.ws.onmessage = (event) => {
                    const message = JSON.parse(event.data);
                    this.handlePokerEvent(message);
                };
                
                this.ws.onclose = () => {
                    this.logMessage('🔴 Disconnected from poker server');
                };
            }
            
            joinGame() {
                this.ws.send(JSON.stringify({
                    type: 'join_game',
                    game_id: this.gameId,
                    session_token: this.sessionToken,
                    public_key: this.publicKey
                }));
            }
            
            handlePokerEvent(event) {
                switch (event.type) {
                    case 'player_joined':
                        this.logMessage(`🎯 ${event.player} joined the game`);
                        this.updatePlayerSeats(event.players_count);
                        break;
                        
                    case 'hand_started':
                        this.logMessage('🎴 New hand started!');
                        this.clearCommunityCards();
                        this.showPlayerActions();
                        break;
                        
                    case 'cards_dealt':
                        this.updateCommunityCards(event.community_cards);
                        break;
                        
                    case 'player_action':
                        this.logMessage(`🎲 ${event.player} ${event.action}${event.amount ? ' ' + event.amount : ''}`);
                        break;
                        
                    case 'hand_complete':
                        this.logMessage(`🏆 ${event.winner} wins ${event.pot} KAS!`);
                        this.hidePlayerActions();
                        break;
                }
            }
            
            fold() {
                this.sendAction('fold');
                this.hidePlayerActions();
            }
            
            call() {
                this.sendAction('call');
            }
            
            showRaiseDialog() {
                document.getElementById('raise-dialog').style.display = 'block';
            }
            
            raise() {
                const amount = parseInt(document.getElementById('raise-amount').value);
                if (amount > 0) {
                    this.sendAction('raise', amount);
                    document.getElementById('raise-dialog').style.display = 'none';
                }
            }
            
            sendAction(action, amount = null) {
                this.ws.send(JSON.stringify({
                    type: 'player_action',
                    game_id: this.gameId,
                    action: action,
                    amount: amount,
                    public_key: this.publicKey
                }));
            }
            
            updatePlayerSeats(playerCount) {
                // Update UI to show current players
                document.getElementById('player-seats').innerHTML = '';
                for (let i = 0; i < playerCount; i++) {
                    const seat = document.createElement('div');
                    seat.className = 'player-seat';
                    seat.innerHTML = `
                        <div>Player ${i + 1}</div>
                        <div>1000 KAS</div>
                    `;
                    seat.style.top = `${20 + i * 60}px`;
                    seat.style.left = `${50 + i * 120}px`;
                    document.getElementById('player-seats').appendChild(seat);
                }
            }
            
            updateCommunityCards(cards) {
                const container = document.getElementById('community-cards');
                container.innerHTML = '';
                
                cards.forEach(card => {
                    const cardDiv = document.createElement('div');
                    cardDiv.className = 'card';
                    cardDiv.textContent = `${this.cardRankToString(card.rank)}${this.cardSuitToEmoji(card.suit)}`;
                    container.appendChild(cardDiv);
                });
            }
            
            showPlayerActions() {
                document.getElementById('player-actions').style.display = 'block';
            }
            
            hidePlayerActions() {
                document.getElementById('player-actions').style.display = 'none';
            }
            
            logMessage(message) {
                const log = document.getElementById('log-messages');
                const msgDiv = document.createElement('div');
                msgDiv.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
                log.appendChild(msgDiv);
                log.scrollTop = log.scrollHeight;
            }
            
            cardRankToString(rank) {
                const ranks = { 2: '2', 3: '3', 4: '4', 5: '5', 6: '6', 7: '7', 8: '8', 9: '9', 10: '10', 11: 'J', 12: 'Q', 13: 'K', 14: 'A' };
                return ranks[rank] || '?';
            }
            
            cardSuitToEmoji(suit) {
                const suits = { 'Hearts': '♥️', 'Diamonds': '♦️', 'Clubs': '♣️', 'Spades': '♠️' };
                return suits[suit] || '?';
            }
        }
        
        const poker = new KaspaPoker();
    </script>
</body>
</html>
```

---

## 📋 **Phase 5: Testing and Integration (45 minutes)**

### 5.1 End-to-End Testing (30 min)
- [ ] Start kaspa-auth server: `cargo run -- http-peer`
- [ ] Start poker server: `cd examples/kaspa-poker && cargo run`
- [ ] Open poker frontend: Browser → `poker.html`
- [ ] Test authentication flow: Auth popup → session token
- [ ] Test poker game: Join game → play hands → verify actions
- [ ] Test multiple players: Open multiple browser tabs

### 5.2 Integration Verification (15 min)
- [ ] Verify session tokens from kaspa-auth work in poker
- [ ] Test session expiry/revocation affects poker game
- [ ] Confirm blockchain transactions for both auth and poker
- [ ] Check real-time updates work across all clients

---

## 🎉 **Success Criteria**

You'll know Session 4 is complete when:

1. **Multi-player poker works**: 2+ players can join and play
2. **Authentication required**: Can't join without valid kaspa-auth session
3. **Real-time updates**: All players see actions immediately
4. **Blockchain integration**: Poker actions recorded on Kaspa
5. **Session management**: Auth expiry/revocation affects poker access

---

## 💭 **Why This Session is Revolutionary**

1. **Proves Architecture Scales**: Complex applications work with kaspa-auth
2. **Demonstrates P2P Gaming**: True decentralized poker tournament
3. **Shows Integration Pattern**: How other apps can use kaspa-auth
4. **Market Validation**: Real use case for blockchain authentication

**Quote**: *"When developers see poker tournaments working on Kaspa, they'll understand the potential"*

This session transforms kaspa-auth from "authentication demo" to "foundation for P2P applications"! 🏆