// Copyright 2023 Lexi Robinson
// Licensed under the EUPL-1.2

pub mod application_layer;
pub mod error;
pub mod link_layer;
pub mod transport_layer;
pub mod types;

#[cfg(test)]
mod test_parse {
	use rstest::rstest;
	use winnow::prelude::*;
	use winnow::Bytes;

	use crate::parse::error::MBusError;
	use crate::parse::link_layer::Packet;
	use crate::utils::fancy_error;
	use crate::utils::read_test_file;

	#[rstest]
	fn test_libmbus_test_frames(
		#[values(
			"abb_delta.hex",
			"abb_f95.hex",
			"ACW_Itron-BM-plus-m.hex",
			"ACW_Itron-CYBLE-M-Bus-14.hex",
			"allmess_cf50.hex",
			"amt_calec_mb.hex",
			"berg_dz_plus.hex",
			"eastron_sdm630.hex",
			"EDC.hex",
			"EFE_Engelmann-Elster-SensoStar-2.hex",
			"EFE_Engelmann-WaterStar.hex",
			"ELS_Elster-F96-Plus.hex",
			"els_falcon.hex",
			"Elster-F2.hex",
			"els_tmpa_telegramm1.hex",
			"ELV-Elvaco-CMa10.hex",
			"elv_temp_humid.hex",
			"emh_diz.hex",
			"EMU_EMU-Professional-375-M-Bus.hex",
			"engelmann_sensostar2c.hex",
			"example_data_01.hex",
			"example_data_02.hex",
			"filler.hex",
			"FIN-Finder-7E.23.8.230.0020.hex",
			"frame1.hex",
			"frame2.hex",
			"gmc_emmod206.hex",
			"GWF-MTKcoder.hex",
			"itron_bm_+m.hex",
			"itron_cf_51.hex",
			"itron_cf_55.hex",
			"itron_cf_echo_2.hex",
			"itron_cyble_m-bus_v1.4_cold_water.hex",
			"itron_cyble_m-bus_v1.4_gas.hex",
			"itron_cyble_m-bus_v1.4_water.hex",
			"itron_integral_mk_maxx.hex",
			"kamstrup_382_005.hex",
			"kamstrup_multical_601.hex",
			"landis+gyr_ultraheat_t230.hex",
			"LGB_G350.hex",
			"manual_frame3.hex",
			"manual_frame7.hex",
			"metrona_pollutherm.hex",
			"metrona_ultraheat_xs.hex",
			"minol_minocal_c2.hex",
			"minol_minocal_wr3.hex",
			"nzr_dhz_5_63.hex",
			"oms_frame1.hex",
			"oms_frame2.hex",
			"oms_frame3.hex",
			"ram_modularis.hex",
			"rel_padpuls2.hex",
			"rel_padpuls3.hex",
			"REL-Relay-Padpuls2.hex",
			"SBC_Saia-Burgess-ALE3.hex",
			"sen_pollucom_e.hex",
			"SEN_Pollustat.hex",
			"sen_pollutherm.hex",
			"SEN_Sensus-PolluStat-E.hex",
			"SEN_Sensus-PolluTherm.hex",
			"siemens_rvd235.hex",
			"siemens_water.hex",
			"siemens_wfh21.hex",
			"SLB_CF-Compact-Integral-MK-MaXX.hex",
			"sontex_supercal_531_telegram1.hex",
			"svm_f22_telegram1.hex",
			"tch_telegramm1.hex",
			"tecson.hex",
			"THI_cma10.hex",
			"wmbus-converted.hex",
			"ZRM_Minol-Minocal-C2.hex"
			// TODO: These are using the compact frame
			// "manual_frame2.hex",
			// "sen_pollusonic_2.hex",
		)]
		filename: &str,
	) -> Result<(), MBusError> {
		let data = read_test_file(&format!("./libmbus_test_data/test-frames/{filename}"))
			.expect("test file must be valid");

		let result = Packet::parse.parse(Bytes::new(&data[..]));
		match result {
			Ok(_) => Ok(()),
			Err(e) => {
				let e = e.into_inner();
				eprint!("{filename} failed: ");
				fancy_error(&e);
				Err(e)
			}
		}
	}
}
