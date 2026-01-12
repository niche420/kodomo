package com.kodomo

import android.content.Intent
import android.os.Bundle
import android.widget.*
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    private lateinit var serverAddressInput: EditText
    private lateinit var connectButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        serverAddressInput = findViewById(R.id.server_address_input)
        connectButton = findViewById(R.id.connect_button)

        // Pre-fill with common default
        serverAddressInput.setText("192.168.1.100:8080")

        connectButton.setOnClickListener {
            val serverAddress = serverAddressInput.text.toString().trim()

            if (serverAddress.isEmpty()) {
                Toast.makeText(this, "Please enter server address", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            // Validate format (basic check)
            if (!serverAddress.contains(":")) {
                Toast.makeText(this, "Format: IP:PORT (e.g., 192.168.1.100:8080)", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            val intent = Intent(this, StreamingActivity::class.java).apply {
                putExtra("server_address", serverAddress)
            }
            startActivity(intent)
        }
    }
}