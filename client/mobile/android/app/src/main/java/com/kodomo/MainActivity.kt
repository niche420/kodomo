package com.kodomo

import android.content.Intent
import android.os.Bundle
import android.widget.*
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    private lateinit var serverAddressInput: EditText
    private lateinit var tailscaleHostnameInput: EditText
    private lateinit var useTailscaleCheckbox: CheckBox
    private lateinit var connectButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        serverAddressInput = findViewById(R.id.server_address_input)
        tailscaleHostnameInput = findViewById(R.id.tailscale_hostname_input)
        useTailscaleCheckbox = findViewById(R.id.use_tailscale_checkbox)
        connectButton = findViewById(R.id.connect_button)

        connectButton.setOnClickListener {
            val intent = Intent(this, StreamingActivity::class.java).apply {
                putExtra("server_address", serverAddressInput.text.toString())
                putExtra("use_tailscale", useTailscaleCheckbox.isChecked)
                putExtra("tailscale_hostname", tailscaleHostnameInput.text.toString())
            }
            startActivity(intent)
        }
    }
}
