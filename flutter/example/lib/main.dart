import 'package:flutter/material.dart';
import 'package:kizuna/kizuna.dart';

void main() {
  runApp(const KizunaExampleApp());
}

class KizunaExampleApp extends StatelessWidget {
  const KizunaExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kizuna Example',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const KizunaHomePage(),
    );
  }
}

class KizunaHomePage extends StatefulWidget {
  const KizunaHomePage({super.key});

  @override
  State<KizunaHomePage> createState() => _KizunaHomePageState();
}

class _KizunaHomePageState extends State<KizunaHomePage> {
  late Kizuna _kizuna;
  bool _initialized = false;
  List<PeerInfo> _peers = [];
  String _status = 'Not initialized';
  PlatformInfo? _platformInfo;

  @override
  void initState() {
    super.initState();
    _initializeKizuna();
  }

  Future<void> _initializeKizuna() async {
    try {
      setState(() {
        _status = 'Initializing...';
      });

      // Get platform information
      _platformInfo = PlatformInfo.getCurrentPlatform();

      // Create Kizuna instance
      _kizuna = Kizuna();

      // Configure Kizuna
      final config = KizunaConfig(
        deviceName: 'Flutter Example Device',
        userName: 'Flutter User',
        enableMdns: true,
        enableUdp: true,
        enableBluetooth: false,
        enableEncryption: true,
        requireAuthentication: true,
        trustMode: 'manual',
        enableIpv6: true,
        enableQuic: true,
        enableWebrtc: true,
        enableWebsocket: true,
      );

      // Initialize
      await _kizuna.initialize(config);

      setState(() {
        _initialized = true;
        _status = 'Initialized successfully';
      });
    } catch (e) {
      setState(() {
        _status = 'Initialization failed: $e';
      });
    }
  }

  Future<void> _discoverPeers() async {
    if (!_initialized) {
      setState(() {
        _status = 'Please initialize first';
      });
      return;
    }

    try {
      setState(() {
        _status = 'Discovering peers...';
      });

      final peers = await _kizuna.discoverPeers();

      setState(() {
        _peers = peers;
        _status = 'Found ${peers.length} peer(s)';
      });
    } catch (e) {
      setState(() {
        _status = 'Discovery failed: $e';
      });
    }
  }

  Future<void> _connectToPeer(String peerId) async {
    try {
      setState(() {
        _status = 'Connecting to peer...';
      });

      final connection = await _kizuna.connectToPeer(peerId);

      setState(() {
        _status = 'Connected to ${connection.peerId}';
      });
    } catch (e) {
      setState(() {
        _status = 'Connection failed: $e';
      });
    }
  }

  @override
  void dispose() {
    if (_initialized) {
      _kizuna.shutdown();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('Kizuna Example'),
      ),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Platform Information Card
            if (_platformInfo != null)
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Platform Information',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 8),
                      Text('Platform: ${_platformInfo!.platform}'),
                      Text('Version: ${_platformInfo!.version}'),
                      const SizedBox(height: 8),
                      Text(
                        'Supported Features:',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                              fontWeight: FontWeight.bold,
                            ),
                      ),
                      ...(_platformInfo!.supportedFeatures.map(
                        (feature) => Text('  • $feature'),
                      )),
                    ],
                  ),
                ),
              ),
            const SizedBox(height: 16),

            // Status Card
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Status',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    Text(_status),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 16),

            // Action Buttons
            ElevatedButton(
              onPressed: _initialized ? _discoverPeers : null,
              child: const Text('Discover Peers'),
            ),
            const SizedBox(height: 16),

            // Peers List
            Expanded(
              child: Card(
                child: _peers.isEmpty
                    ? const Center(
                        child: Text('No peers discovered yet'),
                      )
                    : ListView.builder(
                        itemCount: _peers.length,
                        itemBuilder: (context, index) {
                          final peer = _peers[index];
                          return ListTile(
                            title: Text(peer.name),
                            subtitle: Text('ID: ${peer.id}'),
                            trailing: IconButton(
                              icon: const Icon(Icons.connect_without_contact),
                              onPressed: () => _connectToPeer(peer.id),
                            ),
                          );
                        },
                      ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
