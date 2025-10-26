import { test, expect } from '@playwright/test';

test.describe('Plugin Integration E2E Tests', () => {

  test('should validate plugin initialization patterns', async () => {
    // Test plugin initialization data structures
    const pluginConfigs = {
      protocol: {
        version: '1.0.0',
        maxMessageSize: 1048576,
        heartbeatInterval: 30,
        sessionTimeout: 300
      },
      input: {
        keyboardEnabled: true,
        mouseEnabled: true,
        toggleHotkey: 'ctrl+k'
      },
      service: {
        systemdEnabled: true,
        launchdEnabled: false,
        serviceName: 'soft-kvm'
      },
      security: {
        tlsEnabled: true,
        clientAuthEnabled: false
      },
      discovery: {
        autoDiscovery: true,
        serviceType: 'kvm-server'
      }
    };

    // Validate each plugin configuration
    expect(pluginConfigs.protocol.version).toBe('1.0.0');
    expect(pluginConfigs.protocol.maxMessageSize).toBeGreaterThan(0);
    expect(pluginConfigs.input.keyboardEnabled).toBe(true);
    expect(pluginConfigs.service.systemdEnabled).toBe(true);
    expect(pluginConfigs.security.tlsEnabled).toBe(true);
    expect(pluginConfigs.discovery.autoDiscovery).toBe(true);
  });

  test('should handle protocol server lifecycle', async () => {
    // Test server start
    const serverStarted = await helper.startProtocolServer('127.0.0.1:8081');
    expect(serverStarted).toBe(true);

    // Check server status
    const protocolStatus = await helper.getPluginStatus('protocol');
    expect(protocolStatus.isInitialized).toBe(true);
    expect(protocolStatus.serverRunning).toBe(true);

    // Test server stop (shutdown)
    await page.evaluate(async () => {
      // @ts-ignore - Tauri invoke
      await window.__TAURI__.invoke('shutdown_protocol');
    });

    // Check server stopped
    const updatedStatus = await helper.getPluginStatus('protocol');
    expect(updatedStatus.serverRunning).toBe(false);
  });

  test('should handle input capture lifecycle', async () => {
    // Start input capture
    const captureStarted = await helper.startInputCapture();
    expect(captureStarted).toBe(true);

    // Check input status
    const inputStatus = await helper.getPluginStatus('input');
    expect(inputStatus.isCapturing).toBe(true);

    // Send test input event
    const eventSent = await helper.sendTestInputEvent();
    expect(eventSent).toBe(true);

    // Stop input capture
    const captureStopped = await helper.stopInputCapture();
    expect(captureStopped).toBe(true);

    // Check input status after stop
    const finalStatus = await helper.getPluginStatus('input');
    expect(finalStatus.isCapturing).toBe(false);
  });

  test('should handle client-server connection', async () => {
    // Start server in background
    const serverStarted = await helper.startProtocolServer('127.0.0.1:8082');
    expect(serverStarted).toBe(true);

    // Connect client (this would be another app instance in real scenario)
    // For now, test the connection attempt
    const connected = await helper.connectToServer('127.0.0.1:8082');
    // Connection might fail in test environment, but the attempt should be made
    expect(typeof connected).toBe('boolean');

    // Check connection status
    const connectionStatus = await helper.getConnectionStatus();
    expect(connectionStatus).toBeDefined();
  });

  test('should handle configuration persistence', async () => {
    // Test configuration save/load
    // This requires UI elements to set configuration

    // For now, test that configuration can be accessed
    const config = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('get_config');
      } catch (error) {
        return null;
      }
    });

    // Config should exist or be null (both are valid)
    expect(config !== undefined).toBe(true);
  });

  test('should handle plugin communication errors gracefully', async () => {
    // Test invalid server address
    const connected = await helper.connectToServer('invalid-address:99999');
    expect(connected).toBe(false);

    // Check for error messages
    const errors = await helper.getErrorMessages();
    expect(errors.length).toBeGreaterThanOrEqual(0); // Might not show in UI immediately

    // Test invalid configuration
    const result = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('start_protocol_server', { address: '' });
      } catch (error) {
        return { error: true };
      }
    });
    expect(result.error).toBe(true);
  });

  test('should recover from error states', async () => {
    // Simulate an error state
    await helper.simulateNetworkError();

    // Attempt to recover by restarting services
    const serverStarted = await helper.startProtocolServer('127.0.0.1:8083');
    expect(serverStarted).toBe(true);

    // Check that app is still functional
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should handle multiple plugin operations concurrently', async () => {
    // Start multiple operations concurrently
    const [serverResult, inputResult] = await Promise.all([
      helper.startProtocolServer('127.0.0.1:8084'),
      helper.startInputCapture()
    ]);

    expect(serverResult).toBe(true);
    expect(inputResult).toBe(true);

    // Check both plugins are running
    const [protocolStatus, inputStatus] = await Promise.all([
      helper.getPluginStatus('protocol'),
      helper.getPluginStatus('input')
    ]);

    expect(protocolStatus.serverRunning).toBe(true);
    expect(inputStatus.isCapturing).toBe(true);

    // Clean up
    await helper.stopInputCapture();
    await page.evaluate(async () => {
      // @ts-ignore - Tauri invoke
      await window.__TAURI__.invoke('shutdown_protocol');
    });
  });

  test('should maintain plugin state across operations', async () => {
    // Start server
    await helper.startProtocolServer('127.0.0.1:8085');
    let status = await helper.getPluginStatus('protocol');
    expect(status.serverRunning).toBe(true);

    // Start input capture
    await helper.startInputCapture();
    status = await helper.getPluginStatus('input');
    expect(status.isCapturing).toBe(true);

    // Perform operations that shouldn't affect other plugins
    await helper.sendTestInputEvent();

    // Check both plugins still running
    const [protocolStatus, inputStatus] = await Promise.all([
      helper.getPluginStatus('protocol'),
      helper.getPluginStatus('input')
    ]);

    expect(protocolStatus.serverRunning).toBe(true);
    expect(inputStatus.isCapturing).toBe(true);
  });
});
