package com.kodomo

import android.content.Intent
import android.os.Bundle
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.kodomo.R

/**
 * Main activity - connection setup screen
 */
class MainActivity : AppCompatActivity() {

    private lateinit var serverAddressInput: EditText
    private lateinit var tailscaleHostnameInput: EditText
    private lateinit var useTailscaleCheckbox: CheckBox
    private lateinit var connectButton: Button

    companion object {
        const val TAG = "MainActivity"
        private const val PREF_NAME = "kodomo_prefs"
        private const val KEY_SERVER_ADDRESS = "server_address"
        private const val KEY_USE_TAILSCALE = "use_tailscale"
        private const val KEY_TAILSCALE_HOSTNAME = "tailscale_hostname"
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        initViews()
        loadPreferences()
        setupListeners()
    }

    private fun initViews() {
        serverAddressInput = findViewById(R.id.server_address_input)
        tailscaleHostnameInput = findViewById(R.id.tailscale_hostname_input)
        useTailscaleCheckbox = findViewById(R.id.use_tailscale_checkbox)
        connectButton = findViewById(R.id.connect_button)
    }

    private fun loadPreferences() {
        val prefs = getSharedPreferences(PREF_NAME, MODE_PRIVATE)

        serverAddressInput.setText(
            prefs.getString(KEY_SERVER_ADDRESS, "192.168.1.100:8080")
        )

        tailscaleHostnameInput.setText(
            prefs.getString(KEY_TAILSCALE_HOSTNAME, "kodomo-server")
        )

        useTailscaleCheckbox.isChecked = prefs.getBoolean(KEY_USE_TAILSCALE, false)

        updateTailscaleInputVisibility()
    }

    private fun savePreferences() {
        val prefs = getSharedPreferences(PREF_NAME, MODE_PRIVATE)
        prefs.edit().apply {
            putString(KEY_SERVER_ADDRESS, serverAddressInput.text.toString())
            putString(KEY_TAILSCALE_HOSTNAME, tailscaleHostnameInput.text.toString())
            putBoolean(KEY_USE_TAILSCALE, useTailscaleCheckbox.isChecked)
            apply()
        }
    }

    private fun setupListeners() {
        useTailscaleCheckbox.setOnCheckedChangeListener { _, _ ->
            updateTailscaleInputVisibility()
        }

        connectButton.setOnClickListener {
            onConnectClicked()
        }
    }

    private fun updateTailscaleInputVisibility() {
        tailscaleHostnameInput.isEnabled = useTailscaleCheckbox.isChecked
    }

    private fun onConnectClicked() {
        val useTailscale = useTailscaleCheckbox.isChecked

        val serverAddress = if (useTailscale) {
            val hostname = tailscaleHostnameInput.text.toString().trim()
            if (hostname.isEmpty()) {
                Toast.makeText(this, "Please enter Tailscale hostname", Toast.LENGTH_SHORT).show()
                return
            }
            hostname
        } else {
            val address = serverAddressInput.text.toString().trim()
            if (address.isEmpty()) {
                Toast.makeText(this, "Please enter server address", Toast.LENGTH_SHORT).show()
                return
            }
            address
        }

        // Save preferences
        savePreferences()

        // Start streaming activity
        val intent = Intent(this, StreamingActivity::class.java).apply {
            putExtra(StreamingActivity.EXTRA_SERVER_ADDRESS, serverAddress)
            putExtra(StreamingActivity.EXTRA_USE_TAILSCALE, useTailscale)
            putExtra(StreamingActivity.EXTRA_TAILSCALE_HOSTNAME,
                tailscaleHostnameInput.text.toString())
        }

        startActivity(intent)
    }
}