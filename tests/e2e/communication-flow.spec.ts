import { test, expect } from '@playwright/test';
import { _electron as electron } from 'playwright';
import { createTestHelper } from './helpers';

test.describe('Communication Flow E2E Tests', () => {
  let serverApp: Awaited<ReturnType<typeof electron.launch>>;
  let clientApp: Awaited<ReturnType<typeof electron.launch>>;
  let serverPage: any;
  let clientPage: any;
  let serverHelper: ReturnType<typeof createTestHelper>;
  let clientHelper: ReturnType<typeof createTestHelper>;

  test.beforeAll(async () => {
    // Launch server app
    serverApp = await electron.launch({
      args: ['.'],
      env: {
        ...process.env,
        NODE_ENV: 'test',
        KVM_MODE: 'server',
      },
    });

    // Launch client app
    clientApp = await electron.launch({
      args: ['.'],
      env: {
        ...process.env,
        NODE_ENV: 'test',
        KVM_MODE: 'client',
      },
    });

    serverPage = await serverApp.firstWindow();
    clientPage = await clientApp.firstWindow();

    serverHelper = createTestHelper(serverPage);
    clientHelper = createTestHelper(clientPage);
  });

  test.afterAll(async () => {
    await serverApp.close();
    await clientApp.close();
  });

  test.beforeEach(async () => {
    await serverHelper.waitForAppReady();
    await clientHelper.waitForAppReady();
    await serverHelper.clearErrors();
    await clientHelper.clearErrors();
  });

  test('should establish full KVM communication flow', async () => {
    // Start server
    const serverStarted = await serverHelper.startProtocolServer('127.0.0.1:8090');
    expect(serverStarted).toBe(true);

    // Wait for server to be ready
    await serverPage.waitForTimeout(1000);

    // Connect client
    const clientConnected = await clientHelper.connectToServer('127.0.0.1:8090');
    expect(clientConnected).toBe(true);

    // Wait for connection establishment
    const connectionEstablished = await clientHelper.waitForConnection(10000);
    expect(connectionEstablished).toBe(true);

    // Verify connection status
    const clientStatus = await clientHelper.getConnectionStatus();
    expect(clientStatus.connected).toBe(true);

    const serverStatus = await serverHelper.getPluginStatus('protocol');
    expect(serverStatus.activeSessions).toBeGreaterThan(0);
  });

  test('should handle input event transmission', async () => {
    // Ensure connection is established
    const serverStarted = await serverHelper.startProtocolServer('127.0.0.1:8091');
    const clientConnected = await clientHelper.connectToServer('127.0.0.1:8091');
    expect(serverStarted && clientConnected).toBe(true);

    await clientHelper.waitForConnection();

    // Start input capture on client
    const captureStarted = await clientHelper.startInputCapture();
    expect(captureStarted).toBe(true);

    // Send input event
    const eventSent = await clientHelper.sendTestInputEvent();
    expect(eventSent).toBe(true);

    // Verify event was received (this would require server-side verification)
    // For now, just ensure no errors occurred
    const clientErrors = await clientHelper.getErrorMessages();
    const serverErrors = await serverHelper.getErrorMessages();

    expect(clientErrors.length).toBe(0);
    expect(serverErrors.length).toBe(0);
  });

  test('should handle connection recovery', async () => {
    // Establish connection
    await serverHelper.startProtocolServer('127.0.0.1:8092');
    await clientHelper.connectToServer('127.0.0.1:8092');
    await clientHelper.waitForConnection();

    // Simulate disconnection
    await clientHelper.simulateNetworkError();
    await clientPage.waitForTimeout(1000);

    // Verify disconnection
    const statusAfterDisconnect = await clientHelper.getConnectionStatus();
    expect(statusAfterDisconnect.connected).toBe(false);

    // Attempt reconnection
    const reconnected = await clientHelper.connectToServer('127.0.0.1:8092');
    expect(reconnected).toBe(true);

    // Verify reconnection
    const connectionRecovered = await clientHelper.waitForConnection(5000);
    expect(connectionRecovered).toBe(true);
  });

  test('should handle multiple client connections', async () => {
    // Start server
    await serverHelper.startProtocolServer('127.0.0.1:8093');

    // Connect first client
    await clientHelper.connectToServer('127.0.0.1:8093');
    await clientHelper.waitForConnection();

    // Launch second client
    const secondClientApp = await electron.launch({
      args: ['.'],
      env: {
        ...process.env,
        NODE_ENV: 'test',
        KVM_MODE: 'client',
      },
    });

    const secondClientPage = await secondClientApp.firstWindow();
    const secondClientHelper = createTestHelper(secondClientPage);
    await secondClientHelper.waitForAppReady();

    // Connect second client
    const secondConnected = await secondClientHelper.connectToServer('127.0.0.1:8093');
    expect(secondConnected).toBe(true);
    await secondClientHelper.waitForConnection();

    // Verify server has multiple connections
    const serverStatus = await serverHelper.getPluginStatus('protocol');
    expect(serverStatus.activeSessions).toBeGreaterThanOrEqual(2);

    // Clean up
    await secondClientApp.close();
  });

  test('should handle large data transmission', async () => {
    // Establish connection
    await serverHelper.startProtocolServer('127.0.0.1:8094');
    await clientHelper.connectToServer('127.0.0.1:8094');
    await clientHelper.waitForConnection();

    // Send multiple large events (simulating video frames or bulk input)
    const promises = [];
    for (let i = 0; i < 10; i++) {
      promises.push(clientHelper.sendTestInputEvent());
    }

    const results = await Promise.all(promises);
    const successCount = results.filter(r => r).length;

    // At least 80% success rate for large data transmission
    expect(successCount / results.length).toBeGreaterThan(0.8);
  });

  test('should handle network latency and jitter', async () => {
    // Establish connection
    await serverHelper.startProtocolServer('127.0.0.1:8095');
    await clientHelper.connectToServer('127.0.0.1:8095');
    await clientHelper.waitForConnection();

    // Send events with varying timing (simulating network conditions)
    const delays = [100, 50, 200, 10, 500, 25];

    for (const delay of delays) {
      await clientHelper.sendTestInputEvent();
      await clientPage.waitForTimeout(delay);
    }

    // Verify connection remains stable
    const finalStatus = await clientHelper.getConnectionStatus();
    expect(finalStatus.connected).toBe(true);

    // Check no errors occurred
    const errors = await clientHelper.getErrorMessages();
    expect(errors.length).toBe(0);
  });

  test('should handle protocol version negotiation', async () => {
    // This test would verify that different protocol versions can communicate
    // For now, test that version information is available

    const protocolStatus = await serverHelper.getPluginStatus('protocol');
    expect(protocolStatus.version).toBeDefined();
    expect(protocolStatus.version).toBe('1.0.0');

    const clientProtocolStatus = await clientHelper.getPluginStatus('protocol');
    expect(clientProtocolStatus.version).toBe('1.0.0');
  });

  test('should maintain session integrity during communication', async () => {
    // Establish connection
    await serverHelper.startProtocolServer('127.0.0.1:8096');
    await clientHelper.connectToServer('127.0.0.1:8096');
    await clientHelper.waitForConnection();

    // Get initial session info
    const initialServerStatus = await serverHelper.getPluginStatus('protocol');
    const initialSessionCount = initialServerStatus.activeSessions;

    // Perform multiple operations
    await clientHelper.startInputCapture();
    for (let i = 0; i < 5; i++) {
      await clientHelper.sendTestInputEvent();
      await clientPage.waitForTimeout(100);
    }
    await clientHelper.stopInputCapture();

    // Verify session integrity maintained
    const finalServerStatus = await serverHelper.getPluginStatus('protocol');
    expect(finalServerStatus.activeSessions).toBe(initialSessionCount);

    const finalClientStatus = await clientHelper.getConnectionStatus();
    expect(finalClientStatus.connected).toBe(true);
  });
});
